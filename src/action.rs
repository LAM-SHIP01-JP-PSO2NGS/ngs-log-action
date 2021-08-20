use crate::conf::{Action, ActionType};
use crate::error::NgsLogActionError;
use crate::ngs_log::NgsLog;
use crate::{format_datetime, now, CONF};
use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use futures::{future::join_all, FutureExt};
use num_format::{Locale, ToFormattedString};
use once_cell::sync::Lazy;
use std::cmp::{max, Ordering};
use std::collections::HashMap;
use std::ops;
use std::{io::Write, process::Command};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::spawn;
use tokio::sync::Mutex;
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
pub struct Counter {
 pub current: u32,
 pub prev: u32,
}

/// 現在と直前の値を保持するカウンター
impl Counter {
 fn cmp(&self, rhs: &Self) -> Ordering {
  self.current.cmp(&rhs.current)
 }
}

impl ops::AddAssign<u32> for Counter {
 fn add_assign(&mut self, rhs: u32) {
  self.prev = self.current;
  self.current += rhs;
 }
}

pub static ITEM_COUNTER: Lazy<Mutex<HashMap<String, Counter>>> =
 Lazy::new(|| Mutex::new(HashMap::new()));
static ITEM_COUNTER_BEGIN: Lazy<Mutex<DateTime<FixedOffset>>> = Lazy::new(|| Mutex::new(now()));

pub async fn initialize() {
 ITEM_COUNTER.lock().await;
 ITEM_COUNTER_BEGIN.lock().await;
}

pub async fn do_action(
 action: &Action,
 ngs_log: &NgsLog,
 finished_actions: &mut Vec<ActionType>,
) -> Result<()> {
 // action
 let mut futures = Vec::new();
 if action.show == Some(true) && !finished_actions.contains(&ActionType::Show) {
  futures.push(show(&ngs_log).boxed());
  finished_actions.push(ActionType::Show);
 }
 if !finished_actions.contains(&ActionType::Sound) {
  if let Some(ref sound_file_path) = action.sound {
   futures.push(sound(&sound_file_path).boxed());
   finished_actions.push(ActionType::Sound);
  }
 }
 if !finished_actions.contains(&ActionType::Command) {
  if let Some(ref action_command) = action.command {
   futures.push(command(action_command).boxed());
   finished_actions.push(ActionType::Command);
  }
 }
 if !finished_actions.contains(&ActionType::Get) {
  if let Some(ref url) = action.get {
   futures.push(get(&url, &ngs_log).boxed());
   finished_actions.push(ActionType::Get);
  }
 }
 if !finished_actions.contains(&ActionType::Post) {
  if let Some(ref url) = action.post {
   futures.push(post(&url, &ngs_log).boxed());
   finished_actions.push(ActionType::Post);
  }
 }
 if Some(true) == action.count && !finished_actions.contains(&ActionType::Count) {
  futures.push(count(&ngs_log).boxed());
  finished_actions.push(ActionType::Count);
 }
 if Some(true) == action.show_item_counts && !finished_actions.contains(&ActionType::ShowItemCounts)
 {
  futures.push(show_item_counts().boxed());
  finished_actions.push(ActionType::ShowItemCounts);
 }
 if Some(true) == action.reset_item_counts
  && !finished_actions.contains(&ActionType::ResetItemCounts)
 {
  futures.push(reset_item_counts().boxed());
  finished_actions.push(ActionType::ResetItemCounts);
 }

 join_all(futures).await;
 Ok(())
}

pub async fn count(ngs_log: &NgsLog) -> Result<()> {
 if let NgsLog::ItemLog(item_log) = ngs_log {
  *ITEM_COUNTER
   .lock()
   .await
   .entry(item_log.item.clone())
   .or_insert(Counter {
    current: 0,
    prev: 0,
   }) += item_log.count;
 }
 Ok(())
}

pub async fn reset_item_counts() -> Result<()> {
 ITEM_COUNTER.lock().await.clear();
 *ITEM_COUNTER_BEGIN.lock().await = now();
 let mut stdout = StandardStream::stdout(ColorChoice::Always);
 let color = Some(Color::Ansi256(CONF.get_color_ansi256_item()));
 stdout.set_color(ColorSpec::new().set_fg(color))?;
 writeln!(
  &mut stdout,
  "========== アイテムの集計をリセットしました ==========="
 )
 .unwrap();
 Ok(())
}

pub async fn show_item_counts() -> Result<()> {
 let begin = ITEM_COUNTER_BEGIN.lock().await.clone();
 let now = now();
 let dt = now - begin;
 let dt = format!(
  r#"{:02}°{:02}'{:02}""#,
  dt.num_hours(),
  dt.num_minutes(),
  dt.num_seconds()
 );
 let mut stdout = StandardStream::stdout(ColorChoice::Always);
 let color = Some(Color::Ansi256(CONF.get_color_ansi256_item()));
 stdout.set_color(ColorSpec::new().set_fg(color))?;
 writeln!(
  &mut stdout,
  "=== 取得アイテム集計: {} -> {} ( {} ) ===",
  format_datetime(&begin),
  format_datetime(&now),
  dt
 )
 .unwrap();
 let lock = ITEM_COUNTER.lock().await;
 let mut result: Vec<_> = lock.iter().collect();
 result.sort_by(|a, b| b.1.cmp(a.1));
 // 最長文字数を決定
 let mut item_len_max = 0usize;
 let mut count_len_max = 0usize;
 for (item, count) in &result {
  item_len_max = max(item_len_max, UnicodeWidthStr::width(&item[..]));
  count_len_max = max(
   count_len_max,
   count.current.to_formatted_string(&Locale::ja).len(),
  );
 }
 // 出力
 for (item, count) in result {
  let item_unicode_width = UnicodeWidthStr::width(&item[..]);
  let item_padding =
   " ".repeat(std::cmp::max(0i16, item_len_max as i16 - item_unicode_width as i16) as usize);
  writeln!(
   &mut stdout,
   "{}{} × {:>padding_width$}",
   item,
   item_padding,
   count.current,
   padding_width = count_len_max
  )
  .unwrap();
 }
 writeln!(
  &mut stdout,
  "============================================================="
 )
 .unwrap();
 Ok(())
}

pub async fn show(ngs_log: &NgsLog) -> Result<()> {
 let mut stdout = StandardStream::stdout(ColorChoice::Always);
 let color = Some(Color::Ansi256(CONF.get_color_ansi256(&ngs_log)));
 stdout.set_color(ColorSpec::new().set_fg(color))?;

 let column_separator = CONF.get_column_separator();

 let action_pattern_part = if CONF.is_show_action_pattern() {
  format!("[Action::Show]{}", column_separator)
 } else {
  column_separator.clone()
 };

 let datetime_part = format!(
  "{}{}",
  format_datetime(&ngs_log.get_datetime()),
  column_separator
 );

 let channel_stringify = || {
  let channel_padding_width = match CONF.global {
   Some(ref global) => match global.channel_padding_width {
    Some(channel_padding_width) => channel_padding_width,
    _ => 6,
   },
   _ => 6,
  };
  format!(
   "{:<channel_padding_width$}{}",
   ngs_log.get_channel_or_category_string(),
   column_separator,
   channel_padding_width = channel_padding_width as usize
  )
  .to_uppercase()
 };
 let channel_part = match CONF.global {
  Some(ref global) => match global.show_channel {
   Some(show_channel) if !show_channel => "".to_string(),
   _ => channel_stringify(),
  },
  _ => channel_stringify(),
 };

 let name_padding_width = match CONF.global {
  Some(ref global) => match global.name_padding_width {
   Some(name_padding_width) => name_padding_width,
   _ => 30,
  },
  _ => 30,
 };

 let name_unicode_width = UnicodeWidthStr::width(&ngs_log.get_name()[..]);
 let name_padding =
  " ".repeat(std::cmp::max(0i16, name_padding_width as i16 - name_unicode_width as i16) as usize);
 let name_part = format!("{}{}{}", ngs_log.get_name(), name_padding, column_separator);

 writeln!(
  &mut stdout,
  "{}{}{}{}{}",
  action_pattern_part,
  datetime_part,
  channel_part,
  name_part,
  ngs_log.get_body_or_item_with_count(),
 )?;

 Ok(())
}

pub async fn sound(sound_file_path: &str) -> Result<()> {
 let mut stdout = StandardStream::stdout(ColorChoice::Always);
 let color = Some(Color::Ansi256(CONF.get_color_ansi256_system()));
 stdout.set_color(ColorSpec::new().set_fg(color))?;

 writeln!(
  &mut stdout,
  "[Action::Sound]{}{}",
  CONF.get_column_separator(),
  sound_file_path
 )?;

 // let sound_file_path = sound_file_path.clone();
 let mut player = winaudio::wave::Player::from_file(sound_file_path).unwrap();
 let _ = spawn(async move {
  player.play().unwrap();
 });

 Ok(())
}

pub async fn command(command: &Vec<String>) -> Result<()> {
 let mut stdout = StandardStream::stdout(ColorChoice::Always);
 let color = Some(Color::Ansi256(CONF.get_color_ansi256_system()));
 stdout.set_color(ColorSpec::new().set_fg(color))?;

 writeln!(
  &mut stdout,
  "[Action::Command]{}{:?}",
  CONF.get_column_separator(),
  command
 )?;

 if let Some(c) = command.first() {
  if command.len() > 2 {
   let _output = Command::new(c).args(&command[1..]).output()?;
  } else {
   let _output = Command::new(c).output()?;
  }
 }

 Ok(())
}

pub async fn get(url: &str, ngs_log: &NgsLog) -> Result<()> {
 let mut stdout = StandardStream::stdout(ColorChoice::Always);
 let color = Some(Color::Ansi256(CONF.get_color_ansi256_system()));
 stdout.set_color(ColorSpec::new().set_fg(color))?;

 let url = url
  .replace(
   "{body}",
   &urlencoding::encode(&ngs_log.get_body_or_item_with_count()).to_string(),
  )
  .replace(
   "{name}",
   &urlencoding::encode(&ngs_log.get_name()).to_string(),
  )
  .replace(
   "{channel}",
   &urlencoding::encode(&format!(
    "{:?}",
    ngs_log
     .get_channel()
     .map_or("ITEM".to_string(), |c| format!("{:?}", c))
   )),
  )
  .replace(
   "{datetime}",
   &urlencoding::encode(&format!("{:?}", ngs_log.get_datetime())),
  );

 let mut response = surf::get(&url)
  .header("user-agent", "NGS Log Action")
  .await
  .map_err(|_| NgsLogActionError::ErrorCode(500))?;

 match response.content_type() {
  Some(content_type) if content_type.basetype() == "text" => writeln!(
   &mut stdout,
   "[Action::Get]{}{} => {} = {}",
   CONF.get_column_separator(),
   &url,
   response.status(),
   response
    .body_string()
    .await
    .map_err(|_| NgsLogActionError::ErrorCode(501))?
  )?,
  Some(content_type) => writeln!(
   &mut stdout,
   "[Action::Get]{}{} => {} (not a text, mime = {})",
   CONF.get_column_separator(),
   &url,
   response.status(),
   content_type
  )?,
  _ => (),
 }

 Ok(())
}

pub async fn post(url: &str, ngs_log: &NgsLog) -> Result<()> {
 let mut stdout = StandardStream::stdout(ColorChoice::Always);
 let color = Some(Color::Ansi256(CONF.get_color_ansi256_system()));
 stdout.set_color(ColorSpec::new().set_fg(color))?;

 let mut response = surf::post(url)
  .header("user-agent", "NGS Log Action")
  .header("ngs-log-action-name", urlencoding::encode(&ngs_log.get_name()))
  .header("ngs-log-action-channel", format!("{:?}", ngs_log.get_channel().map_or("ITEM".to_string(), |c|format!("{:?}",c))))
  .header("ngs-log-action-datetime", ngs_log.get_datetime().to_string())
  .body(ngs_log.get_body_or_item_with_count())
  .await
  .unwrap()
  // .map_err(|_| NgsLogActionError::ErrorCode(510))?
  ;

 match response.content_type() {
  Some(content_type) if content_type.basetype() == "text" => writeln!(
   &mut stdout,
   "[Action::Post]{}{} => {} = {}",
   CONF.get_column_separator(),
   &url,
   response.status(),
   response
    .body_string()
    .await
    .map_err(|_| NgsLogActionError::ErrorCode(511))?
  )?,
  Some(content_type) => writeln!(
   &mut stdout,
   "[Action::Post]{}{} => {} (not a text, mime = {})",
   CONF.get_column_separator(),
   &url,
   response.status(),
   content_type
  )?,
  _ => (),
 }

 Ok(())
}
