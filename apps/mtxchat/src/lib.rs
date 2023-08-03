pub mod api;
mod url;
mod web;

pub use api::*;
use chat::{Chat, ChatOp};
use locales::t;
use modals::Modals;

use std::fmt::Write as _;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write as StdWrite};
use std::path::PathBuf;

/// PDDB Dict for mtxchat keys
const MTXCHAT_DICT: &str = "mtxchat";

const FILTER_KEY: &str = "_filter";
const PASSWORD_KEY: &str = "password";
const ROOM_ID_KEY: &str = "_room_id";
const ROOM_NAME_KEY: &str = "room_name";
const ROOM_DOMAIN_KEY: &str = "room_domain";
const SINCE_KEY: &str = "_since";
const TOKEN_KEY: &str = "_token";
const USER_ID_KEY: &str = "_user_id";
const USER_NAME_KEY: &str = "user_name";
const USER_DOMAIN_KEY: &str = "user_domain";

const HTTPS: &str = "https://";
const DOMAIN_MATRIX: &str = "matrix.org";

const EMPTY: &str = "";
const MTX_LONG_TIMEOUT: i32 = 60000; // ms

pub const CLOCK_NOT_SET_ID: usize = 1;
pub const PDDB_NOT_MOUNTED_ID: usize = 2;
pub const WIFI_NOT_CONNECTED_ID: usize = 3;
pub const MTXCLI_INITIALIZED_ID: usize = 4;
pub const WIFI_CONNECTED_ID: usize = 5;
pub const SET_USER_ID: usize = 6;
pub const SET_PASSWORD_ID: usize = 7;
pub const LOGGED_IN_ID: usize = 8;
pub const LOGIN_FAILED_ID: usize = 9;
pub const SET_ROOM_ID: usize = 10;
pub const ROOMID_FAILED_ID: usize = 11;
pub const FILTER_FAILED_ID: usize = 12;
pub const SET_SERVER_ID: usize = 13;
pub const LOGGING_IN_ID: usize = 14;
pub const LOGGED_OUT_ID: usize = 15;
pub const NOT_CONNECTED_ID: usize = 16;
pub const FAILED_TO_SEND_ID: usize = 17;
pub const PLEASE_LOGIN_ID: usize = 18;

#[cfg(not(target_os = "xous"))]
pub const HOSTED_MODE: bool = true;
#[cfg(target_os = "xous")]
pub const HOSTED_MODE: bool = false;

//#[derive(Debug)]
pub struct MtxChat<'a> {
    chat: &'a Chat,
    user_id: String,
    user_name: String,
    user_domain: String,
    token: String,
    logged_in: bool,
    room_id: String,
    room_name: String,
    room_domain: String,
    filter: String,
    since: String,
    wifi_connected: bool,
    listening: bool,
    modals: Modals,
}
impl<'a> MtxChat<'a> {
    pub fn new(chat: &Chat) -> MtxChat {
        let xns = xous_names::XousNames::new().unwrap();
        let modals = Modals::new(&xns).expect("can't connect to Modals server");
        let common = MtxChat {
            chat: chat,
            user_id: EMPTY.to_string(),
            user_name: EMPTY.to_string(),
            user_domain: DOMAIN_MATRIX.to_string(),
            token: EMPTY.to_string(),
            logged_in: false,
            room_id: EMPTY.to_string(),
            room_name: EMPTY.to_string(),
            room_domain: EMPTY.to_string(),
            filter: EMPTY.to_string(),
            since: EMPTY.to_string(),
            wifi_connected: false,
            listening: false,
            modals: modals,
        };
        let mut keypath = PathBuf::new();
        keypath.push(MTXCHAT_DICT);
        if std::fs::metadata(&keypath).is_ok() { // keypath exists
             // log::info!("dict '{}' exists", MTXCHAT_DICT);
        } else {
            log::info!("dict '{}' does NOT exist.. creating it", MTXCHAT_DICT);
            match std::fs::create_dir_all(&keypath) {
                Ok(_) => log::info!("created dict: {}", MTXCHAT_DICT),
                Err(e) => log::warn!("failed to create dict: {:?}", e),
            }
        }
        common
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), Error> {
        if key.starts_with("__") {
            Err(Error::new(
                ErrorKind::PermissionDenied,
                "may not set a variable beginning with __ ",
            ))
        } else {
            log::info!("set '{}' = '{}'", key, value);
            let mut keypath = PathBuf::new();
            keypath.push(MTXCHAT_DICT);
            if std::fs::metadata(&keypath).is_ok() { // keypath exists
                 // log::info!("dict '{}' exists", MTXCHAT_DICT);
            } else {
                log::info!("dict '{}' does NOT exist.. creating it", MTXCHAT_DICT);
                std::fs::create_dir_all(&keypath)?;
            }
            keypath.push(key);
            File::create(keypath)?.write_all(value.as_bytes())?;
            match key {
                // update cached values
                FILTER_KEY => self.filter = value.to_string(),
                PASSWORD_KEY => (),
                ROOM_ID_KEY => self.room_id = value.to_string(),
                ROOM_NAME_KEY => self.room_name = value.to_string(),
                ROOM_DOMAIN_KEY => self.room_domain = value.to_string(),
                SINCE_KEY => self.since = value.to_string(),
                USER_NAME_KEY => self.user_name = value.to_string(),
                USER_DOMAIN_KEY => self.user_domain = value.to_string(),
                USER_ID_KEY => self.user_id = value.to_string(),
                _ => {}
            }
            Ok(())
        }
    }

    // will log on error (vs. panic)
    pub fn set_debug(&mut self, key: &str, value: &str) -> bool {
        match self.set(key, value) {
            Ok(()) => true,
            Err(e) => {
                log::info!("error setting key {}: {:?}", key, e);
                false
            }
        }
    }

    pub fn unset(&mut self, key: &str) -> Result<(), Error> {
        if key.starts_with("__") {
            Err(Error::new(
                ErrorKind::PermissionDenied,
                "may not unset a variable beginning with __ ",
            ))
        } else {
            log::info!("unset '{}'", key);
            let mut keypath = PathBuf::new();
            keypath.push(MTXCHAT_DICT);
            if std::fs::metadata(&keypath).is_ok() { // keypath exists
                 // log::info!("dict '{}' exists", MTXCHAT_DICT);
            } else {
                log::info!("dict '{}' does NOT exist.. creating it", MTXCHAT_DICT);
                std::fs::create_dir_all(&keypath)?;
            }
            keypath.push(key);
            if std::fs::metadata(&keypath).is_ok() {
                // keypath exists
                log::info!("dict:key = '{}:{}' exists.. deleting it", MTXCHAT_DICT, key);
                std::fs::remove_file(keypath)?;
            }
            match key {
                // update cached values
                FILTER_KEY => self.filter = EMPTY.to_string(),
                ROOM_ID_KEY => self.room_id = EMPTY.to_string(),
                ROOM_DOMAIN_KEY => self.room_domain = EMPTY.to_string(),
                SINCE_KEY => self.since = EMPTY.to_string(),
                USER_DOMAIN_KEY => self.user_domain = EMPTY.to_string(),
                USER_ID_KEY => self.user_id = EMPTY.to_string(),
                USER_NAME_KEY => self.user_name = EMPTY.to_string(),
                _ => {}
            }
            Ok(())
        }
    }

    // will log on error (vs. panic)
    pub fn unset_debug(&mut self, key: &str) -> bool {
        match self.unset(key) {
            Ok(()) => true,
            Err(e) => {
                log::info!("error unsetting key {}: {:?}", key, e);
                false
            }
        }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, Error> {
        // if key.eq(CURRENT_VERSION_KEY) {
        //     Ok(Some(self.version.clone()))
        // } else {
        let mut keypath = PathBuf::new();
        keypath.push(MTXCHAT_DICT);
        keypath.push(key);
        if let Ok(mut file) = File::open(keypath) {
            let mut value = String::new();
            file.read_to_string(&mut value)?;
            log::info!("get '{}' = '{}'", key, value);
            Ok(Some(value))
        } else {
            Ok(None)
        }
        // }
    }

    pub fn get_or(&mut self, key: &str, default: &str) -> String {
        match self.get(key) {
            Ok(None) => default.to_string(),
            Ok(Some(value)) => value.to_string(),
            Err(e) => {
                log::info!("error getting key {}: {:?}", key, e);
                default.to_string()
            }
        }
    }

    pub fn login(&mut self) -> bool {
        self.token = self.get_or(TOKEN_KEY, EMPTY);
        self.logged_in = false;
        let mut server = String::new();
        write!(
            server,
            "{}{}",
            HTTPS,
            &self.get_or(USER_DOMAIN_KEY, DOMAIN_MATRIX)
        )
        .expect("failed to write server");
        if self.token.len() > 0 {
            if let Some(user_id) = web::whoami(&server, &self.token) {
                self.user_id = user_id;
                self.logged_in = true;
            }
        }
        if !self.logged_in {
            if web::get_login_type(&server) {
                let user_id = self.get_or(USER_ID_KEY, USER_ID_KEY);
                let password = self.get_or(PASSWORD_KEY, EMPTY);
                if let Some(new_token) = web::authenticate_user(&server, &user_id, &password) {
                    self.set_debug(TOKEN_KEY, &new_token);
                    self.user_id = user_id;
                    self.logged_in = true;
                } else {
                    log::info!(
                        "Error: cannnot login with type: {}",
                        web::MTX_LOGIN_PASSWORD
                    );
                }
            }
        }
        if self.logged_in {
            log::info!("logged_in");
        } else {
            log::info!("login failed");
        }
        self.logged_in
    }

    pub fn login_modal(&mut self) {
        const HIDE: &str = "*****";
        let mut builder = self.modals.alert_builder(t!("mtxchat.login.title", locales::LANG));
        let builder = match self.get(USER_NAME_KEY) {
            // TODO add TextValidationFn
            Ok(Some(user)) => builder.field_placeholder_persist(Some(user), None),
            _ => builder.field(Some(t!("mtxchat.user_name", locales::LANG).to_string()), None),
        };
        let builder = match self.get(USER_DOMAIN_KEY) {
            // TODO add TextValidationFn
            Ok(Some(server)) => builder.field_placeholder_persist(Some(server), None),
            _ => builder.field(Some(t!("mtxchat.domain", locales::LANG).to_string()), None),
        };
        let builder = match self.get(PASSWORD_KEY) {
            Ok(Some(pwd)) => builder.field_placeholder_persist(Some(HIDE.to_string()), None),
            _ => builder.field(Some(t!("mtxchat.password", locales::LANG).to_string()), None),
        };
        if let Ok(payloads) = builder.build() {
            self.unset_debug(TOKEN_KEY);
            if let Ok(content) = payloads.content()[0].content.as_str() {
                self.set(USER_NAME_KEY, content)
                    .expect("failed to save username");
            }
            if let Ok(content) = payloads.content()[1].content.as_str() {
                self.set(USER_DOMAIN_KEY, content)
                    .expect("failed to save server");
            }
            if let Ok(content) = payloads.content()[2].content.as_str() {
                if content.ne(HIDE) {
                    self.set(PASSWORD_KEY, content)
                        .expect("failed to save password");
                }
            }
            let mut user_id = String::new();
            write!(user_id, "@{}:{}", self.user_name, self.user_domain);
            self.set(USER_ID_KEY, &user_id)
                .expect("failed to save user");
        }
        log::info!(
            "# user = '{}' user_name = '{}' server = '{}'",
            self.user_id,
            self.user_name,
            self.user_domain
        );
    }

    // assume logged in, token is valid
    pub fn get_room_id(&mut self) -> bool {
        if self.room_id.len() > 0 {
            true
        } else if self.room_name.len() == 0 {
            false
        } else if self.room_domain.len() == 0 {
            false
        } else {
            let name = self.get_or(ROOM_NAME_KEY, EMPTY);
            let domain = self.get_or(ROOM_DOMAIN_KEY, EMPTY);
            let mut room = String::new();
            write!(
                room,
                "#{}:{}",
                &name,
                &self.get_or(ROOM_DOMAIN_KEY, DOMAIN_MATRIX)
            )
            .expect("failed to write room");
            let mut server = String::new();
            write!(
                server,
                "{}{}",
                HTTPS,
                &self.get_or(USER_DOMAIN_KEY, DOMAIN_MATRIX)
            )
            .expect("failed to write server");
            if let Some(room_id) = web::get_room_id(&server, &room, &self.token) {
                self.set_debug(ROOM_ID_KEY, &room_id);
                true
            } else {
                log::warn!("failed to return room_id");
                false
            }
        }
    }

    pub fn room_modal(&mut self){
        let mut builder = self.modals.alert_builder(t!("mtxchat.room.title", locales::LANG));
        let builder = match self.get(ROOM_NAME_KEY) {
            // TODO add TextValidationFn
            Ok(Some(room)) => builder.field_placeholder_persist(Some(room), None),
            _ => builder.field(Some(t!("mtxchat.room.name", locales::LANG).to_string()), None),
        };
        let builder = match self.get(ROOM_DOMAIN_KEY) {
            // TODO add TextValidationFn
            Ok(Some(server)) => builder.field_placeholder_persist(Some(server), None),
            _ => builder.field(Some(t!("mtxchat.domain", locales::LANG).to_string()), None),
        };
        if let Ok(payloads) = builder.build() {
            self.unset_debug(ROOM_ID_KEY);
            self.unset_debug(SINCE_KEY);
            self.unset_debug(FILTER_KEY);
            if let Ok(content) = payloads.content()[0].content.as_str() {
                self.set(ROOM_NAME_KEY, content)
                    .expect("failed to save server");
            }
            if let Ok(content) = payloads.content()[1].content.as_str() {
                self.set(ROOM_DOMAIN_KEY, content)
                    .expect("failed to save server");
            }
        }
        log::info!(
            "# ROOM_NAME_KEY set '{}' => clearing ROOM_ID_KEY, SINCE_KEY, FILTER_KEY",
            ROOM_NAME_KEY
        );
    }

    // assume logged in, token is valid, room_id is valid, user is valid
    pub fn get_filter(&mut self) -> bool {
        if self.filter.len() > 0 {
            true
        } else {
            let mut user_server = String::new();
            write!(
                user_server,
                "{}{}",
                HTTPS,
                &self.get_or(USER_DOMAIN_KEY, DOMAIN_MATRIX)
            )
            .expect("failed to write server");
                self.set_debug(FILTER_KEY, &new_filter);
                true
            } else {
                false
            }
        }
    }

    pub fn listen(&mut self) {
        if self.listening {
            log::info!("Already listening");
            return;
        }
        if ! self.logged_in {
            log::info!("Not logged in");
            return;
        }
        if self.room_id.len() == 0 {
            if ! self.get_room_id() {
                return;
            }
        }
        if self.filter.len() == 0 {
            if ! self.get_filter() {
                return;
            }
        }
        self.listening = true;
        log::info!("Started listening");
        std::thread::spawn({

            let mut server = String::new();
            write!(
                server,
                "{}{}",
                HTTPS,
                &self.get_or(ROOM_DOMAIN_KEY, DOMAIN_MATRIX)
            )
            .expect("failed to write server");
            let filter = self.filter.clone();
            let since = self.since.clone();
            let room_id = self.room_id.clone();
            let token = self.token.clone();
            // let async_msg_conn = self.async_msg_conn.clone();
            // let async_msg_callback_id = self.async_msg_callback_id.clone();
            move || {
                // log::info!("client_sync for {} ms...", MTX_LONG_TIMEOUT);
                let mut response = String::new();
                // response.push(SENTINEL);
                if let Some((since, messages)) = web::client_sync(&server, &filter, &since, MTX_LONG_TIMEOUT, &room_id, &token) {
                    response.push_str(&since);
                    // response.push(SENTINEL);
                    response.push_str(&messages);
                    // response.push(SENTINEL);
                }
                // let str_buf = StringBuffer::from_str(&response)
                //     .expect("unable to create string message");
                // str_buf.send(async_msg_conn, async_msg_callback_id)
                //     .expect("unable to send string message");
            }
        });
    }

    pub fn listen_over(&mut self, since: &str) {
        self.listening = false;
        log::info!("Stopped listening");
        if since.len() > 0 {
            self.set_debug(SINCE_KEY, since);
            // don't re-start listening if there was an error
            if self.logged_in && (HOSTED_MODE || self.wifi_connected) {
                self.listen();
            }
        }
    }
}

pub(crate) fn heap_usage() -> usize {
    match xous::rsyscall(xous::SysCall::IncreaseHeap(0, xous::MemoryFlags::R))
        .expect("couldn't get heap size")
    {
        xous::Result::MemoryRange(m) => {
            let usage = m.len();
            usage
        }
        _ => {
            log::info!("Couldn't measure heap usage");
            0
        }
    }
}
