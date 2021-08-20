use crate::ngs_log::{NgsLog, NgsLogChannel};
use serde::Deserialize;
use strum_macros::EnumString;

#[derive(Debug, Deserialize)]
pub struct Conf {
 pub global: Option<Global>,
 pub r#if: Option<Vec<If>>,
}

#[derive(Debug, Deserialize)]
pub struct Global {
 pub show_action_pattern: Option<bool>,
 pub datetime_format: Option<String>,
 pub show_channel: Option<bool>,
 pub column_separator: Option<String>,
 pub name_padding_width: Option<u8>,
 pub channel_padding_width: Option<u8>,
 pub color_public: Option<u8>,
 pub color_party: Option<u8>,
 pub color_guild: Option<u8>,
 pub color_group: Option<u8>,
 pub color_reply: Option<u8>,
 pub color_item: Option<u8>,
 pub color_system: Option<u8>,
 pub polling_rate: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct If {
 pub names: Option<Vec<String>>,
 pub channels: Option<Vec<NgsLogChannel>>,
 pub keywords: Option<Vec<String>>,
 pub regex: Option<String>,
 pub ignore_names: Option<Vec<String>>,
 pub ignore_keywords: Option<Vec<String>>,
 pub ignore_regex: Option<String>,
 pub action: Option<Action>,
 pub target: Option<Target>,
 pub item_counts: Option<Vec<ItemCount>>,
}

#[derive(Debug, Deserialize)]
pub struct ItemCount {
 pub keywords: Option<Vec<String>>,
 pub regex: Option<String>,
 pub every: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Action {
 pub show: Option<bool>,
 pub command: Option<Vec<String>>,
 pub get: Option<String>,
 pub post: Option<String>,
 pub sound: Option<String>,
 pub count: Option<bool>,
 pub show_item_counts: Option<bool>,
 pub reset_item_counts: Option<bool>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ActionType {
 Show,
 Command,
 Get,
 Post,
 Sound,
 Count,
 ShowItemCounts,
 ResetItemCounts,
}

#[derive(Debug, EnumString, Deserialize, PartialEq, Eq)]
pub enum Target {
 Chat,
 Item,
}

// Default-Colors
const DC_PUBLIC: u8 = 15;
const DC_PARTY: u8 = 14;
const DC_GUILD: u8 = 172;
const DC_GROUP: u8 = 41;
const DC_REPLY: u8 = 13;
const DC_ITEM: u8 = 227;
const DC_SYSTEM: u8 = 8;
const DEFAULT_POLLING_RATE: f64 = 1.0;

impl Conf {
 pub fn get_polling_rate(&self) -> f64 {
  self.global.as_ref().map_or(DEFAULT_POLLING_RATE, |g| {
   g.polling_rate.unwrap_or(DEFAULT_POLLING_RATE)
  })
 }

 pub fn get_color_ansi256_system(&self) -> u8 {
  self
   .global
   .as_ref()
   .map_or(DC_SYSTEM, |g| g.color_system.unwrap_or(DC_SYSTEM))
 }

 pub fn get_color_ansi256_item(&self) -> u8 {
  self
   .global
   .as_ref()
   .map_or(DC_ITEM, |g| g.color_item.unwrap_or(DC_ITEM))
 }

 pub fn get_color_ansi256(&self, ngs_log: &NgsLog) -> u8 {
  match ngs_log {
   NgsLog::ChatLog(l) => match l.channel {
    NgsLogChannel::Public => self
     .global
     .as_ref()
     .map_or(DC_PUBLIC, |g| g.color_public.unwrap_or(DC_PUBLIC)),
    NgsLogChannel::Party => self
     .global
     .as_ref()
     .map_or(DC_PARTY, |g| g.color_party.unwrap_or(DC_PARTY)),
    NgsLogChannel::Guild => self
     .global
     .as_ref()
     .map_or(DC_GUILD, |g| g.color_guild.unwrap_or(DC_GUILD)),
    NgsLogChannel::Group => self
     .global
     .as_ref()
     .map_or(DC_GROUP, |g| g.color_group.unwrap_or(DC_GROUP)),
    NgsLogChannel::Reply => self
     .global
     .as_ref()
     .map_or(DC_REPLY, |g| g.color_reply.unwrap_or(DC_REPLY)),
   },
   NgsLog::ItemLog(_) => self
    .global
    .as_ref()
    .map_or(DC_ITEM, |g| g.color_item.unwrap_or(DC_ITEM)),
  }
 }

 pub fn is_show_action_pattern(&self) -> bool {
  const DEFAULT_VALUE: bool = false;
  match self.global {
   Some(ref global) => global.show_action_pattern.unwrap_or(DEFAULT_VALUE),
   _ => DEFAULT_VALUE,
  }
 }

 pub fn get_column_separator(&self) -> String {
  const DEFAULT_VALUE: &str = " ";
  match self.global {
   Some(ref global) => global
    .column_separator
    .clone()
    .unwrap_or(DEFAULT_VALUE.to_string()),
   _ => DEFAULT_VALUE.to_string(),
  }
 }
}
