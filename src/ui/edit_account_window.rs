use crate::helpers::ConfigManager;
use crate::main_window::{Display, MainWindow};
use crate::model::{Account, AccountGroup};
use crate::ui::{AccountsWindow, ValidationError};
use gtk::prelude::*;
use gtk::Builder;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct EditAccountWindow {
    pub container: gtk::Box,
    pub input_group: gtk::ComboBoxText,
    pub input_name: gtk::Entry,
    pub input_secret: gtk::TextView,
    pub input_account_id: gtk::Entry,
    pub cancel_button: gtk::Button,
    pub save_button: gtk::Button,
    pub add_accounts_container_edit: gtk::Label,
    pub add_accounts_container_add: gtk::Label,
}

impl EditAccountWindow {
    pub fn new(builder: Builder) -> EditAccountWindow {
        EditAccountWindow {
            container: builder.get_object("edit_account").unwrap(),
            input_group: builder.get_object("edit_account_input_group").unwrap(),
            input_name: builder.get_object("edit_account_input_name").unwrap(),
            input_secret: builder.get_object("edit_account_input_secret").unwrap(),
            input_account_id: builder.get_object("edit_account_input_account_id").unwrap(),
            cancel_button: builder.get_object("edit_account_cancel").unwrap(),
            add_accounts_container_edit: builder.get_object("add_accounts_container_edit").unwrap(),
            add_accounts_container_add: builder.get_object("add_accounts_container_add").unwrap(),
            save_button: builder.get_object("edit_account_save").unwrap(),
        }
    }

    #[allow(clippy::useless_let_if_seq)]
    fn validate(&self) -> Result<(), ValidationError> {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();

        let mut result: Result<(), ValidationError> = Ok(());

        if name.get_buffer().get_text().is_empty() {
            name.set_property_primary_icon_name(Some("gtk-dialog-error"));
            let style_context = name.get_style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError("name".to_owned()));
        }

        let buffer = secret.get_buffer().unwrap();
        let (start, end) = buffer.get_bounds();
        let secret_value: String = match buffer.get_slice(&start, &end, true) {
            Some(secret_value) => secret_value.to_string(),
            None => "".to_owned(),
        };

        if secret_value.is_empty() {
            let style_context = secret.get_style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError("secret".to_owned()));
        } else {
            match Account::generate_time_based_password(secret_value.as_str()) {
                Ok(_) => {}
                Err(_) => {
                    let style_context = secret.get_style_context();
                    style_context.add_class("error");
                    result = Err(ValidationError::FieldError("secret".to_owned()));
                }
            }
        }

        result
    }

    pub fn reset(&self) {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();
        let group = self.input_group.clone();

        name.set_property_primary_icon_name(None);
        let style_context = name.get_style_context();
        style_context.remove_class("error");

        let style_context = secret.get_style_context();
        style_context.remove_class("error");

        let style_context = group.get_style_context();
        style_context.remove_class("error");
    }

    pub fn set_group_dropdown(&self, group_id: Option<u32>, groups: &[AccountGroup]) {
        self.input_group.remove_all();

        groups.iter().for_each(|group| {
            let string = format!("{}", group.id);
            let entry_id = Some(string.as_str());
            self.input_group.append(entry_id, group.name.as_str());

            if group.id == group_id.unwrap_or(0) {
                self.input_group.set_active_id(entry_id);
            }
        });

        // select 1st entry to avoid blank selection choice
        if group_id.is_none() {
            let first_entry = groups.get(0).map(|e| format!("{}", e.id));
            let first_entry = first_entry.as_deref();
            self.input_group.set_active_id(first_entry);
        }
    }

    pub fn edit_account_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        fn with_action<F>(gui: &MainWindow, connection: Arc<Mutex<Connection>>, button: gtk::Button, button_closure: F)
        where
            F: 'static + Fn(Arc<Mutex<Connection>>, &MainWindow) -> Box<dyn Fn(&gtk::Button)>,
        {
            button.connect_clicked(button_closure(connection, gui));
        }

        // CANCEL
        with_action(&gui, connection.clone(), gui.edit_account_window.cancel_button.clone(), |_, gui| {
            let gui = gui.clone();
            Box::new(move |_| {
                let edit_account_window = gui.edit_account_window.clone();
                edit_account_window.reset();
                edit_account_window.input_name.set_text("");

                let buffer = edit_account_window.input_secret.get_buffer().unwrap();
                buffer.set_text("");

                MainWindow::switch_to(&gui, Display::DisplayAccounts);
            })
        });

        // SAVE
        with_action(&gui, connection, gui.edit_account_window.save_button.clone(), |connection, gui| {
            let gui = gui.clone();
            Box::new(move |_| {
                gui.edit_account_window.reset();

                if let Ok(()) = gui.edit_account_window.validate() {
                    let edit_account_window = gui.edit_account_window.clone();

                    let name = edit_account_window.input_name.clone();
                    let secret = edit_account_window.input_secret.clone();
                    let account_id = edit_account_window.input_account_id.clone();
                    let group = edit_account_window.input_group.clone();

                    let name: String = name.get_buffer().get_text();

                    let buffer = secret.get_buffer().unwrap();
                    let (start, end) = buffer.get_bounds();
                    let secret: String = match buffer.get_slice(&start, &end, true) {
                        Some(secret_value) => secret_value.to_string(),
                        None => "".to_owned(),
                    };

                    let group_id = group.get_active_id().unwrap().as_str().to_owned().parse().unwrap();

                    {
                        let connection = connection.lock().unwrap();

                        match account_id.get_buffer().get_text().parse() {
                            Ok(account_id) => {
                                let mut account = Account::new(account_id, group_id, name.as_str(), secret.as_str());

                                ConfigManager::update_account(&connection, &mut account).unwrap();
                            }
                            Err(_) => {
                                let mut account = Account::new(0, group_id, name.as_str(), secret.as_str());

                                ConfigManager::save_account(&connection, &mut account).unwrap();
                            }
                        };
                    }

                    AccountsWindow::replace_accounts_and_widgets(&gui, connection.clone());

                    edit_account_window.reset();
                    MainWindow::switch_to(&gui, Display::DisplayAccounts);
                }
            })
        });
    }
}
