use chrono::{DateTime, FixedOffset};
use num_format::{Locale, ToFormattedString};
use serde::Deserialize;
use strum_macros::EnumString;

#[derive(Debug, EnumString, Deserialize, PartialEq, Eq)]
pub enum NgsLogChannel {
 /// = 白
 #[strum(serialize = "PUBLIC")]
 #[serde(rename = "PUBLIC")]
 Public,
 /// = 青
 #[strum(serialize = "PARTY")]
 #[serde(rename = "PARTY")]
 Party,
 /// = 橙 Team
 #[strum(serialize = "GUILD")]
 #[serde(rename = "GUILD")]
 Guild,
 /// = 紫 Whisper
 #[strum(serialize = "REPLY")]
 #[serde(rename = "REPLY")]
 Reply,
 /// = 緑
 #[strum(serialize = "GROUP")]
 #[serde(rename = "GROUP")]
 Group,
}

#[derive(Debug)]
pub struct ChatLog {
 pub datetime: DateTime<FixedOffset>,
 pub log_id: u16,
 pub channel: NgsLogChannel,
 pub player_id: u32,
 pub name: String,
 pub body: String,
}

#[derive(Debug, EnumString, Deserialize, PartialEq, Eq)]
pub enum ItemCategory {
 #[strum(serialize = "PICKUP")]
 #[serde(rename = "PICKUP")]
 Pickup,
 #[strum(serialize = "REWARD")]
 #[serde(rename = "REWARD")]
 Reward,
}

#[derive(Debug)]
pub struct ItemLog {
 pub datetime: DateTime<FixedOffset>,
 pub log_id: u16,
 pub category: ItemCategory,
 pub player_id: u32,
 pub name: String,
 pub item: String,
 pub count: u32,
}

#[derive(Debug)]
pub enum NgsLog {
 ChatLog(ChatLog),
 ItemLog(ItemLog),
}

impl NgsLog {
 pub fn get_datetime(&self) -> &DateTime<FixedOffset> {
  match self {
   NgsLog::ChatLog(log) => &log.datetime,
   NgsLog::ItemLog(log) => &log.datetime,
  }
 }
 pub fn get_channel(&self) -> Option<&NgsLogChannel> {
  match self {
   NgsLog::ChatLog(log) => Some(&log.channel),
   NgsLog::ItemLog(_) => None,
  }
 }
 pub fn get_channel_or_category_string(&self) -> String {
  match self {
   NgsLog::ChatLog(log) => format!("{:?}", log.channel),
   NgsLog::ItemLog(log) => format!("{:?}", log.category),
  }
 }
 pub fn get_name(&self) -> &String {
  match self {
   NgsLog::ChatLog(log) => &log.name,
   NgsLog::ItemLog(log) => &log.name,
  }
 }
 // pub fn get_body(&self) -> Option<&String> {
 //  match self {
 //   NgsLog::ChatLog(log) => Some(&log.body),
 //   NgsLog::ItemLog(_) => None,
 //  }
 // }
 pub fn append_body(&mut self, s: &str) -> Option<()> {
  match self {
   NgsLog::ChatLog(log) => {
    log.body = format!("{}\n{}", log.body, s);
    Some(())
   }
   NgsLog::ItemLog(_) => None,
  }
 }
 pub fn get_body_or_item(&self) -> &String {
  match self {
   NgsLog::ChatLog(log) => &log.body,
   NgsLog::ItemLog(log) => &log.item,
  }
 }
 pub fn get_body_or_item_with_count(&self) -> String {
  match self {
   NgsLog::ChatLog(log) => log.body.clone(),
   NgsLog::ItemLog(log) => format!(
    "{} × {}",
    log.item,
    log.count.to_formatted_string(&Locale::ja)
   ),
  }
 }
}
