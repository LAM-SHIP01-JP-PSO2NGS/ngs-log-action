use anyhow::Result;
use chrono::{
 offset::{Offset, TimeZone},
 DateTime, FixedOffset, Local,
};
use dir::home_dir;
use encoding_rs_io::DecodeReaderBytes;
use once_cell::sync::Lazy;
use std::{
 cmp::max,
 fs::{self, File},
 io::{BufRead, BufReader},
 path::PathBuf,
 process::Command,
 str::FromStr,
};

mod action;
mod conf;
mod error;
mod ngs_log;

use conf::{ActionType, Conf, If, Target};
use error::NgsLogActionError;
use ngs_log::{ChatLog, ItemCategory, ItemLog, NgsLog, NgsLogChannel};

static CONF: Lazy<Conf> = Lazy::new(|| {
 let conf_str = fs::read_to_string("conf.toml").unwrap();
 let conf = toml::from_str(&conf_str).unwrap();
 conf
});

#[tokio::main]
async fn main() -> Result<()> {
 let mut last_log_datetime = now();

 action::initialize().await;
 let polling_sleep = 1.0 / CONF.get_polling_rate();

 println!(
  "[System]{}NGS Log Action {} 起動 {}",
  CONF.get_column_separator(),
  env!("CARGO_PKG_VERSION"),
  format_datetime(&last_log_datetime)
 );

 loop {
  {
   let ngs_logs = get_new_logs(last_log_datetime).await?;
   if !ngs_logs.is_empty() {
    for ngs_log in &ngs_logs {
     apply_ngs_log_actions(&ngs_log).await?;
    }
    last_log_datetime = ngs_logs
     .last()
     .ok_or(NgsLogActionError::ErrorCode(300))?
     .get_datetime()
     .clone();
   }
  }

  tokio::time::sleep(tokio::time::Duration::from_secs_f64(polling_sleep)).await;
 }

 // Ok(())
}

fn now() -> DateTime<FixedOffset> {
 let tz_offset = Local.timestamp(0, 0).offset().fix();
 Local::now().with_timezone(&tz_offset)
}

fn format_datetime(datetime: &DateTime<FixedOffset>) -> String {
 match CONF.global {
  Some(ref global) => match global.datetime_format {
   Some(ref datetime_format) if datetime_format.is_empty() => "".to_string(),
   Some(ref datetime_format) => datetime.format(datetime_format).to_string(),
   _ => datetime.to_string(),
  },
  _ => datetime.to_string(),
 }
}

async fn apply_ngs_log_actions(ngs_log: &NgsLog) -> Result<()> {
 let mut finished_actions = Vec::new();
 if let Some(r#if) = &CONF.r#if {
  for r#if in r#if {
   apply_log_action(r#if, ngs_log, &mut finished_actions).await?;
  }
 }
 Ok(())
}

async fn apply_log_action(
 r#if: &If,
 ngs_log: &NgsLog,
 finished_actions: &mut Vec<ActionType>,
) -> Result<()> {
 // filters
 if let Some(ref target) = r#if.target {
  match ngs_log {
   NgsLog::ChatLog(_) if target.ne(&Target::Chat) => return Ok(()),
   NgsLog::ItemLog(_) if target.ne(&Target::Item) => return Ok(()),
   _ => {}
  }
 }
 if let Some(ref channels) = r#if.channels {
  if let Some(channel) = ngs_log.get_channel() {
   if !channels.contains(channel) {
    return Ok(());
   }
  }
 }
 if let Some(ref names) = r#if.names {
  let name = ngs_log.get_name();
  if !names.contains(name) {
   return Ok(());
  }
 }
 if let Some(ref keywords) = r#if.keywords {
  if !keywords
   .iter()
   .any(|keyword| ngs_log.get_body_or_item().find(keyword).is_some())
  {
   return Ok(());
  }
 }
 if let Some(ref regex) = r#if.regex {
  let regex = regex::Regex::new(regex)?;
  if !regex.is_match(&ngs_log.get_body_or_item()) {
   return Ok(());
  }
 }

 // ignore- series
 if let Some(ref ignore_names) = r#if.ignore_names {
  if ignore_names.contains(&ngs_log.get_name()) {
   return Ok(());
  }
 }
 if let Some(ref ignore_keywords) = r#if.ignore_keywords {
  if ignore_keywords
   .iter()
   .any(|ignore_keyword| ngs_log.get_body_or_item().find(ignore_keyword).is_some())
  {
   return Ok(());
  }
 }
 if let Some(ref ignore_regex) = r#if.ignore_regex {
  let ignore_regex = regex::Regex::new(ignore_regex)?;
  if ignore_regex.is_match(&ngs_log.get_body_or_item()) {
   return Ok(());
  }
 }

 if let Some(ref item_counts) = r#if.item_counts {
  let item_counter = action::ITEM_COUNTER.lock().await.clone();
  for (i, c) in item_counter.iter() {
   for p in item_counts {
    if let Some(every) = p.every {
     if c.prev / every >= c.current / every {
      continue;
     }
    }
    if let Some(ref ks) = p.keywords {
     if !ks.iter().any(|k| i.contains(k)) {
      continue;
     }
    }
    if let Some(ref re) = p.regex {
     let re = regex::Regex::new(re)?;
     if !re.is_match(i) {
      continue;
     }
    }
    if let Some(ref action) = r#if.action {
     action::do_action(action, &ngs_log, finished_actions).await?;
    }
   }
  }
 } else if let Some(ref action) = r#if.action {
  action::do_action(action, &ngs_log, finished_actions).await?;
 }
 Ok(())
}

fn parse_datetime(datetime_string: &str) -> Result<DateTime<FixedOffset>> {
 let tz_offset = Local.timestamp(0, 0).offset().fix();
 let datetime_string = format!("{}{:?}", datetime_string, &tz_offset);
 let datetime = DateTime::parse_from_rfc3339(&datetime_string)?;
 Ok(datetime)
}

async fn get_ngs_user_directory_path() -> Result<PathBuf> {
 let home_directory = home_dir().ok_or(NgsLogActionError::ErrorCode(1))?;
 let ngs_user_directory_path = home_directory
  .join("Documents")
  .join("SEGA")
  .join("PHANTASYSTARONLINE2");
 Ok(ngs_user_directory_path)
}

async fn get_ngs_logs_directory_path() -> Result<PathBuf> {
 let ngs_logs_directory_path = get_ngs_user_directory_path().await?.join("log_ngs");
 Ok(ngs_logs_directory_path)
}

async fn get_pso2_logs_directory_path() -> Result<PathBuf> {
 let ngs_logs_directory_path = get_ngs_user_directory_path().await?.join("log");
 Ok(ngs_logs_directory_path)
}

/// https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/issues/1
async fn last_modified_fix(path: &PathBuf) -> Result<()> {
 if let Some(path_str) = path.to_str() {
  let _output = Command::new("cmd")
   .args(&["/c", "dir", "/A", "/R", "/Q", path_str])
   .output()
   .unwrap();
 }
 Ok(())
}

/// return Result<( Chat, Action, Reward )>
async fn get_latest_log_file_paths() -> Result<(Option<PathBuf>, Option<PathBuf>, Option<PathBuf>)>
{
 let ngs_logs_path = get_ngs_logs_directory_path().await?;
 last_modified_fix(&ngs_logs_path).await?;
 let ngs_directory_entries = fs::read_dir(ngs_logs_path)?;
 let mut pso2ngs_directory_entries: Vec<_> = ngs_directory_entries.map(|a| a.unwrap()).collect();

 let pso2_logs_path = get_pso2_logs_directory_path().await?;
 last_modified_fix(&pso2_logs_path).await?;
 let pso2_directory_entries = fs::read_dir(pso2_logs_path)?;
 let mut pso2_directory_entries: Vec<_> = pso2_directory_entries.map(|a| a.unwrap()).collect();

 pso2ngs_directory_entries.append(&mut pso2_directory_entries);

 pso2ngs_directory_entries.sort_by(|a, b| {
  let a = a.metadata().unwrap().modified().unwrap();
  let b = b.metadata().unwrap().modified().unwrap();
  b.cmp(&a)
 });

 let chat = pso2ngs_directory_entries
  .iter()
  .find(|a| a.file_name().to_string_lossy().starts_with("ChatLog"))
  .map(|e| e.path());

 let action = pso2ngs_directory_entries
  .iter()
  .find(|a| a.file_name().to_string_lossy().starts_with("ActionLog"))
  .map(|e| e.path());

 let reward = pso2ngs_directory_entries
  .iter()
  .find(|a| a.file_name().to_string_lossy().starts_with("RewardLog"))
  .map(|e| e.path());

 Ok((chat, action, reward))
}

fn create_reader_from_path(path: PathBuf) -> Result<BufReader<DecodeReaderBytes<File, Vec<u8>>>> {
 let file = fs::OpenOptions::new()
  .read(true)
  .write(false)
  .create(false)
  .open(path)?;

 let reader = DecodeReaderBytes::new(file);
 let reader = BufReader::new(reader);
 Ok(reader)
}

async fn get_latest_log_readers() -> Result<(
 Option<BufReader<DecodeReaderBytes<File, Vec<u8>>>>,
 Option<BufReader<DecodeReaderBytes<File, Vec<u8>>>>,
 Option<BufReader<DecodeReaderBytes<File, Vec<u8>>>>,
)> {
 let (chat, action, reward) = get_latest_log_file_paths().await?;
 let chat = chat.map(|p| create_reader_from_path(p).unwrap());
 let action = action.map(|p| create_reader_from_path(p).unwrap());
 let reward = reward.map(|p| create_reader_from_path(p).unwrap());
 Ok((chat, action, reward))
}

fn unescape_double_quote(s: &str) -> String {
 s.replacen(r#""""#, r#"""#, usize::MAX)
}

fn pre_unescape_double_quote(s: &str) -> String {
 s.replacen(r#""""#, "\t", usize::MAX)
}

fn finish_unescape_double_quote(s: &str) -> String {
 s.replacen("\t", r#"""#, usize::MAX)
}

async fn get_new_chat_logs(
 reader: Option<BufReader<DecodeReaderBytes<File, Vec<u8>>>>,
 last_datetime: &DateTime<FixedOffset>,
) -> Result<Vec<NgsLog>> {
 let mut ngs_logs = Vec::new();

 if let Some(chat) = reader {
  let lines = chat.lines();

  for line in lines {
   if let Ok(line) = line {
    match extract_datetime(&line) {
     // 過去ログ
     Ok((datetime, _)) if &datetime <= last_datetime => (),
     // 新規ログ
     Ok((datetime, tail)) if &datetime > last_datetime => {
      let mut tail = tail.split("\t");
      let log_id = tail.next().unwrap().parse()?;
      let channel = NgsLogChannel::from_str(tail.next().unwrap()).unwrap();
      let player_id = tail.next().unwrap().parse()?;
      let name = tail.next().unwrap().to_string();
      let mut body = unescape_double_quote(tail.next().unwrap());
      // 複数行の最初の行
      if body == r#"""#{
       body = "\n".to_string();
      }
      if body.starts_with(r#"""#) && body.char_indices().nth(1).unwrap().1 != '"' {
       body = body[1..].to_string();
      }
      ngs_logs.push(NgsLog::ChatLog(ChatLog {
       datetime,
       log_id,
       channel,
       player_id,
       name,
       body,
      }))
     }
     // 新規ログまたは新規ログの2行目以降
     _ => {
      if let Some(last_log) = ngs_logs.last_mut() {
       let line = pre_unescape_double_quote(&line);
       // 新規ログの2行目以降
       if line.chars().last() == Some('"') {
        // 複数行の最後の行( " で終端 )
        let line = finish_unescape_double_quote(&line[..line.len() - 1]);
        last_log.append_body(&line);
        // (*last_log).body = format!("{}\n{}", last_log.body, line)
       } else {
        // 複数行の途中の行
        last_log.append_body(&finish_unescape_double_quote(&line));
       }
      } else {
       // 前回検出した最後のログが複数行だった場合
       ()
      }
     }
    }
   }
  }
 }

 Ok(ngs_logs)
}

async fn get_new_action_logs(
 reader: Option<BufReader<DecodeReaderBytes<File, Vec<u8>>>>,
 last_datetime: &DateTime<FixedOffset>,
) -> Result<Vec<NgsLog>> {
 let mut ngs_logs = Vec::new();
 if let Some(action) = reader {
  let lines = action.lines();

  for line in lines {
   if let Ok(line) = line {
    match extract_datetime(&line) {
     // 過去ログ
     Ok((datetime, _)) if &datetime <= last_datetime => (),
     // 新規ログ
     Ok((datetime, tail)) if &datetime > last_datetime => {
      let mut tail = tail.split("\t");
      let log_id = tail.next().unwrap().parse()?;
      let category_string = tail.next().unwrap();
      match category_string {
       "[Pickup]" => {
        let category = ItemCategory::Pickup;
        let player_id = tail.next().unwrap().parse()?;
        let name = tail.next().unwrap().to_string();
        let mut item = tail.next().unwrap().to_string();
        let count = match item.is_empty() {
         true => {
          // 例: 2021-08-19T20:40:56	250	[Pickup]	15161621	L,A.M.		Meseta(12)	CurrentMeseta(26029094)
          let buffer = tail.next().unwrap().to_string();
          if buffer.starts_with("Meseta") {
           item = "Meseta".to_string();
           buffer[7..buffer.len() - 1].parse().unwrap()
          } else {
           panic!();
          }
         }
         false => {
          if let Some(buffer) = tail.next() {
           let buffer = buffer.to_string();
           if buffer.starts_with("CurrentNum") {
            1
           } else {
            // 例: 2021-08-19T20:40:17	243	[Pickup]	15161621	L,A.M.	N-グラインダー	Num(1)
            //     2021-08-19T20:55:51	406	[Pickup]	15161621	L,A.M.	ツヴィアダガー	attr:NONE(0)
            let num_begin = buffer.find("(").unwrap() + 1;
            max(1, buffer[num_begin..buffer.len() - 1].parse().unwrap())
           }
          } else {
           // 例: 2021-08-19T20:40:56	249	[Pickup]	15161621	L,A.M.	ツヴィアアーマ
           1
          }
         }
        };
        ngs_logs.push(NgsLog::ItemLog(ItemLog {
         datetime,
         log_id,
         category,
         player_id,
         name,
         item,
         count,
        }))
       }
       _ => {}
      };
     }
     _ => {}
    }
   }
  }
 }

 Ok(ngs_logs)
}

async fn get_new_reward_logs(
 reader: Option<BufReader<DecodeReaderBytes<File, Vec<u8>>>>,
 last_datetime: &DateTime<FixedOffset>,
) -> Result<Vec<NgsLog>> {
 let mut ngs_logs = Vec::new();
 if let Some(action) = reader {
  let lines = action.lines();

  for line in lines {
   if let Ok(line) = line {
    match extract_datetime(&line) {
     // 過去ログ
     Ok((datetime, _)) if &datetime <= last_datetime => (),
     // 新規ログ
     Ok((datetime, tail)) if &datetime > last_datetime => {
      let mut tail = tail.split("\t");
      let category = ItemCategory::Reward;
      let log_id = tail.next().unwrap().parse().unwrap();
      let _unknown = tail.next().unwrap();
      let player_id = 0; // TODO
      let name = tail.next().unwrap().to_string();
      let switcher = tail.next().unwrap();
      match switcher {
       "Meseta" => {
        let item = "Meseta".to_string();
        let count = tail.next().unwrap();
        let count_begin = count.find("(");
        let count_end = count.find(")");
        if count_begin.is_some() && count_end.is_some() {
         let count = (&count[count_begin.unwrap() + 1..count_end.unwrap()])
          .parse()
          .unwrap();
         ngs_logs.push(NgsLog::ItemLog(ItemLog {
          datetime,
          log_id,
          category,
          player_id,
          name,
          item,
          count,
         }))
        }
       }
       "Backpack" => {
        let item = tail.next().unwrap().to_string();
        let count = tail.next().unwrap();
        let count_begin = count.find("(");
        let count_end = count.find(")");
        if count_begin.is_some() && count_end.is_some() {
         let count = (&count[count_begin.unwrap() + 1..count_end.unwrap()])
          .parse()
          .unwrap();
         ngs_logs.push(NgsLog::ItemLog(ItemLog {
          datetime,
          log_id,
          category,
          player_id,
          name,
          item,
          count,
         }))
        }
       }
       _ => {}
      }
     }
     _ => {}
    }
   }
  }
 }

 Ok(ngs_logs)
}

async fn get_new_logs(last_datetime: DateTime<FixedOffset>) -> Result<Vec<NgsLog>> {
 let mut ngs_logs = Vec::new();

 let (chat, action, reward) = get_latest_log_readers().await?;
 let mut chat = get_new_chat_logs(chat, &last_datetime).await?;
 let mut action = get_new_action_logs(action, &last_datetime).await?;
 let mut reward = get_new_reward_logs(reward, &last_datetime).await?;
 ngs_logs.append(&mut chat);
 ngs_logs.append(&mut action);
 ngs_logs.append(&mut reward);
 ngs_logs.sort_by(|a, b| a.get_datetime().cmp(b.get_datetime()));

 Ok(ngs_logs)
}

fn extract_datetime(line: &str) -> Result<(DateTime<FixedOffset>, String)> {
 let (first_column, tail) = line
  .split_once("\t")
  .ok_or(NgsLogActionError::ErrorCode(200))?;
 let datetime = parse_datetime(first_column)?;
 Ok((datetime, tail.to_string()))
}
