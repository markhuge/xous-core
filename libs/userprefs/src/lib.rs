use bincode::error::{DecodeError, EncodeError};
use pddb::Pddb;
use std::io::Read;
use std::io::Write;

static PREFS_DICT: &str = "UserPrefsDict";

// Time-related consts

/// This is the UTC offset from the current hardware RTC reading. This should be fixed once time is set.
const TIME_SERVER_UTC_OFFSET: &'static str = "utc_offset";
/// This is the offset from UTC to the display time zone. This can vary when the user changes time zones.
const TIME_SERVER_TZ_OFFSET: &'static str = "tz_offset";

#[derive(Debug)]
pub enum Error {
    EncodeError(EncodeError),
    DecodeError(DecodeError),
    IoError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<EncodeError> for Error {
    fn from(err: EncodeError) -> Self {
        Self::EncodeError(err)
    }
}

impl From<DecodeError> for Error {
    fn from(err: DecodeError) -> Self {
        Self::DecodeError(err)
    }
}

/// UserPrefs defines the set of all the user preference toggles available for Precursor.
/// Setters and getters are PDDB-aware, and are generated by the `prefsgenerator::GetterSetter` macro.
/// To add a new preference setting just drop a new struct entry in here.
/// The struct field type must be serializable by bincode.
#[derive(prefsgenerator::GetterSetter)]
#[allow(dead_code)] // Allowing dead code here because UserPrefs is used to generate getter/setters.
pub struct UserPrefs {
    pub wifi_kill: bool,
    pub connect_known_networks_on_boot: bool,
    pub autobacklight_on_boot: bool,
    pub autobacklight_timeout: u64,
    pub autosleep_timeout: u64,
    pub autounmount_timeout: u64,
    pub audio_enabled: bool,
    pub earpiece_volume: u32,
    pub headset_volume: u32,
}

pub struct Manager {
    pddb_handle: Pddb,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            pddb_handle: Pddb::new(),
        }
    }

    fn pddb_store_key(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        match self.pddb_handle.get(
            PREFS_DICT,
            key,
            Some(pddb::PDDB_DEFAULT_SYSTEM_BASIS),
            true,
            true,
            None,
            None::<fn()>,
        ) {
            Ok(mut data) => match data.write(value) {
                Ok(_) => Ok(self.pddb_handle.sync().unwrap_or(())),
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e.into()),
        }
    }

    fn pddb_get_key(&self, key: &str) -> Result<Vec<u8>, Error> {
        match self.pddb_handle.get(
            PREFS_DICT,
            key,
            Some(pddb::PDDB_DEFAULT_SYSTEM_BASIS),
            true,
            true,
            None,
            None::<fn()>,
        ) {
            Ok(mut record) => {
                let mut data = Vec::<u8>::new();
                record.read_to_end(&mut data)?;
                Ok(data)
            }
            Err(e) => return Err(e.into()),
        }
    }

    pub fn store_i64(&self, value: i64, key: &str) -> Result<(), Error> {
        let offset_bytes = value.to_le_bytes();

        self.pddb_store_key(key, &offset_bytes)
    }
}

// This impl block is here because some toggles/data (like date/time stuff) needs particular
// serialization/deserialization routines.
impl Manager {
    pub fn timezone_offset(&self) -> Result<Option<i64>, Error> {
        let tz_set_key = self.pddb_get_key(TIME_SERVER_TZ_OFFSET)?;

        if tz_set_key.len() != 8 {
            return Ok(None);
        }

        let sl = &tz_set_key[..];
        let sl: [u8; 8] = sl.try_into().unwrap();

        return Ok(Some(i64::from_le_bytes(sl)));
    }

    pub fn set_timezone_offset(&self, offset: i64) -> Result<(), Error> {
        self.store_i64(offset, TIME_SERVER_TZ_OFFSET)
    }

    pub fn utc_offset(&self) -> Result<i64, Error> {
        let utc_set_key = self.pddb_get_key(TIME_SERVER_UTC_OFFSET)?;

        if utc_set_key.len() != 8 {
            return Ok(0);
        }

        let sl = &utc_set_key[..];
        let sl: [u8; 8] = sl.try_into().unwrap();

        return Ok(i64::from_le_bytes(sl));
    }

    pub fn set_utc_offset(&self, offset: i64) -> Result<(), Error> {
        self.store_i64(offset, TIME_SERVER_UTC_OFFSET)
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}