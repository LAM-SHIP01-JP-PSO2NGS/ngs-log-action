use anyhow::Result;
use chrono::{
 offset::{Offset, TimeZone},
 DateTime, FixedOffset, Local,
};
use dir::home_dir;
use encoding_rs_io::DecodeReaderBytes;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::io::Write;
use std::{
 fs::{self, File},
 io::{BufRead, BufReader},
 path::PathBuf,
 process::Command,
 str::FromStr,
};
use strum_macros::EnumString;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use thiserror::Error;
use toml;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Error)]
enum NgsLogActionError {
 #[error("error-code: {0}")]
 ErrorCode(u32),
}

#[derive(Debug, EnumString, Deserialize, PartialEq, Eq)]
enum NgsLogChannel {
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
struct NgsLog {
 datetime: DateTime<FixedOffset>,
 log_id: u16,
 channel: NgsLogChannel,
 player_id: u32,
 name: String,
 body: String,
}

#[derive(Debug, Deserialize)]
struct Conf {
 global: Option<Global>,
 r#if: Option<Vec<If>>,
}

#[derive(Debug, Deserialize)]
struct Global {
 show_action_pattern: Option<bool>,
 datetime_format: Option<String>,
 show_channel: Option<bool>,
 column_separator: Option<String>,
 name_padding_width: Option<u8>,
 channel_padding_width: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct If {
 names: Option<Vec<String>>,
 channels: Option<Vec<NgsLogChannel>>,
 keywords: Option<Vec<String>>,
 regex: Option<String>,
 ignore_names: Option<Vec<String>>,
 ignore_keywords: Option<Vec<String>>,
 ignore_regex: Option<String>,
 action: Option<Action>,
}

#[derive(Debug, Deserialize)]
struct Action {
 show: Option<bool>,
 command: Option<Vec<String>>,
 get: Option<String>,
 post: Option<String>,
 sound: Option<String>,
}

impl Conf {
 fn is_show_action_pattern(&self) -> bool {
  const DEFAULT_VALUE: bool = false;
  match self.global {
   Some(ref global) => global.show_action_pattern.unwrap_or(DEFAULT_VALUE),
   _ => DEFAULT_VALUE,
  }
 }

 fn get_column_separator(&self) -> String {
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

static CONF: Lazy<Conf> = Lazy::new(|| {
 let conf_str = fs::read_to_string("conf.toml").unwrap();
 let conf = toml::from_str(&conf_str).unwrap();
 conf
});

#[tokio::main]
async fn main() -> Result<()> {
 // println!("conf={:?}", CONF.r#if);
 let mut last_datetime = get_last_datetime().await?;
 println!("[System]{}NGS Log Action 起動", CONF.get_column_separator());
 // println!("[Debug] last_datetime: {:?}", last_datetime);
 loop {
  let new_lines = get_new_lines(last_datetime).await?;
  if !new_lines.is_empty() {
   for ngs_log in &new_lines {
    apply_log_actions(&ngs_log).await?;
   }
   last_datetime = new_lines
    .last()
    .ok_or(NgsLogActionError::ErrorCode(300))?
    .datetime;
  }
  tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
 }

 // Ok(())
}

async fn apply_log_actions(ngs_log: &NgsLog) -> Result<()> {
 // println!("[Debug] A new logging detected: {:?}", ngs_log);
 if let Some(r#if) = &CONF.r#if {
  for r#if in r#if {
   apply_log_action(r#if, ngs_log).await?;
  }
 }
 Ok(())
}

async fn apply_log_action(r#if: &If, ngs_log: &NgsLog) -> Result<()> {
 // println!("[Debug] Apply 'if': {:?}", r#if);

 // filters
 if let Some(ref channels) = r#if.channels {
  if !channels.contains(&ngs_log.channel) {
   return Ok(());
  }
 }
 if let Some(ref names) = r#if.names {
  if !names.contains(&ngs_log.name) {
   return Ok(());
  }
 }
 if let Some(ref keywords) = r#if.keywords {
  if !keywords
   .iter()
   .any(|keyword| ngs_log.body.find(keyword).is_some())
  {
   return Ok(());
  }
 }
 if let Some(ref regex) = r#if.regex {
  let regex = regex::Regex::new(regex)?;
  if !regex.is_match(&ngs_log.body) {
   return Ok(());
  }
 }

 // ignore- series
 if let Some(ref ignore_names) = r#if.ignore_names {
  if ignore_names.contains(&ngs_log.name) {
   return Ok(());
  }
 }
 if let Some(ref ignore_keywords) = r#if.ignore_keywords {
  // println!("ignore_keywords {:?}", ignore_keywords);
  if ignore_keywords
   .iter()
   .any(|ignore_keyword| ngs_log.body.find(ignore_keyword).is_some())
  {
   return Ok(());
  }
 }
 if let Some(ref ignore_regex) = r#if.ignore_regex {
  let ignore_regex = regex::Regex::new(ignore_regex)?;
  if ignore_regex.is_match(&ngs_log.body) {
   return Ok(());
  }
 }

 // action
 if let Some(ref action) = r#if.action {
  match action.show {
   Some(show) if show => {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    let color = Some(match ngs_log.channel {
     NgsLogChannel::Public => Color::White,
     NgsLogChannel::Party => Color::Cyan,
     NgsLogChannel::Guild => Color::Yellow,
     NgsLogChannel::Reply => Color::Magenta,
     NgsLogChannel::Group => Color::Green,
    });
    stdout.set_color(ColorSpec::new().set_fg(color))?;

    let column_separator = CONF.get_column_separator();

    let action_pattern_part = if CONF.is_show_action_pattern() {
     format!("[Action::Show]{}", column_separator)
    } else {
     column_separator.clone()
    };

    let datetime_part = match CONF.global {
     Some(ref global) => match global.datetime_format {
      Some(ref datetime_format) if datetime_format.is_empty() => "".to_string(),
      Some(ref datetime_format) => format!(
       "{}{}",
       ngs_log.datetime.format(datetime_format),
       column_separator
      ),
      _ => format!("{:?}{}", ngs_log.datetime, column_separator),
     },
     _ => format!("{:?}{}", ngs_log.datetime, column_separator),
    };

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
      format!("{:?}", ngs_log.channel),
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

    let name_unicode_width = UnicodeWidthStr::width(&ngs_log.name[..]);
    let name_padding = " "
     .repeat(std::cmp::max(0i16, name_padding_width as i16 - name_unicode_width as i16) as usize);
    let name_part = format!("{}{}{}", ngs_log.name, name_padding, column_separator);

    writeln!(
     &mut stdout,
     "{}{}{}{}{}",
     action_pattern_part, datetime_part, channel_part, name_part, ngs_log.body
    )?;
   }
   _ => (),
  }
  if let Some(ref sound_file_path) = action.sound {
   println!(
    "[Action::Sound]{}{}",
    CONF.get_column_separator(),
    sound_file_path
   );
   // 仕方ないので暫定措置として最終手段っぽい方法で鳴らしておく
   let main_arg = format!(
    "(New-Object Media.SoundPlayer \"{}\").PlaySync()",
    sound_file_path
   );
   let _output = Command::new("powershell")
    .args(&["-c", &main_arg])
    .output()?;

   // println!("output={:?}", output);

   // winaudio は Err は出ないが音も出なかった
   // let mut player = winaudio::wave::Player::from_file(sound_file_path)?;
   // player.play()?;

   // rodio では NoDeivce で死んだ
   // let (_, h) = rodio::OutputStream::try_default()?;
   // let sink = rodio::Sink::try_new(&h)?;
   // let file = File::open(sound_file_path)?;
   // sink.append(rodio::Decoder::new(BufReader::new(file))?);
   // sink.sleep_until_end();
  }
  if let Some(ref command) = action.command {
   println!(
    "[Action::Command]{}{:?}",
    CONF.get_column_separator(),
    command
   );
   if let Some(c) = command.first() {
    if command.len() > 2 {
     let _output = Command::new(c).args(&command[1..]).output()?;
    } else {
     let _output = Command::new(c).output()?;
    }
   }
  }
  if let Some(ref get) = action.get {
   let url = get
    .replace("{body}", &urlencoding::encode(&ngs_log.body).to_string())
    .replace("{name}", &urlencoding::encode(&ngs_log.name).to_string())
    .replace(
     "{channel}",
     &urlencoding::encode(&format!("{:?}", ngs_log.channel)),
    )
    .replace(
     "{datetime}",
     &urlencoding::encode(&format!("{:?}", ngs_log.datetime)),
    );
   let mut response = surf::get(&url)
    .header("user-agent", "NGS Log Action")
    .await
    .map_err(|_| NgsLogActionError::ErrorCode(500))?;
   match response.content_type() {
    Some(content_type) if content_type.basetype() == "text" => println!(
     "[Action::Get]{}{} => {} = {}",
     CONF.get_column_separator(),
     &url,
     response.status(),
     response
      .body_string()
      .await
      .map_err(|_| NgsLogActionError::ErrorCode(501))?
    ),
    Some(content_type) => println!(
     "[Action::Get]{}{} => {} (not a text, mime = {})",
     CONF.get_column_separator(),
     &url,
     response.status(),
     content_type
    ),
    _ => (),
   }
  }
  if let Some(ref post) = action.post {
   let url = post;
   // println!("[Debug]\tpost url={}", url);
   let mut response = surf::post(url)
    .header("user-agent", "NGS Log Action")
    .header("ngs-log-action-name", urlencoding::encode(&ngs_log.name))
    .header("ngs-log-action-channel", format!("{:?}", ngs_log.channel))
    .header("ngs-log-action-datetime", ngs_log.datetime.to_string())
    .body(ngs_log.body.clone())
    .await
    .unwrap()
    // .map_err(|_| NgsLogActionError::ErrorCode(510))?
    ;
   match response.content_type() {
    Some(content_type) if content_type.basetype() == "text" => println!(
     "[Action::Post]{}{} => {} = {}",
     CONF.get_column_separator(),
     &url,
     response.status(),
     response
      .body_string()
      .await
      .map_err(|_| NgsLogActionError::ErrorCode(511))?
    ),
    Some(content_type) => println!(
     "[Action::Post]{}{} => {} (not a text, mime = {})",
     CONF.get_column_separator(),
     &url,
     response.status(),
     content_type
    ),
    _ => (),
   }
  }
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
async fn get_latest_chat_log_file_path() -> Result<PathBuf> {
 let last_modified_fix = |path: &PathBuf| {
  if let Some(path_str) = path.to_str() {
   let _output = Command::new("cmd")
    .args(&["/c", "dir", "/A", "/R", "/Q", path_str])
    .output()
    .unwrap();
  }
 };

 let ngs_logs_path = get_ngs_logs_directory_path().await?;
 last_modified_fix(&ngs_logs_path);
 let ngs_directory_entries = fs::read_dir(ngs_logs_path)?;
 let mut pso2ngs_directory_entries: Vec<_> = ngs_directory_entries.map(|a| a.unwrap()).collect();

 let pso2_logs_path = get_pso2_logs_directory_path().await?;
 last_modified_fix(&pso2_logs_path);
 let pso2_directory_entries = fs::read_dir(pso2_logs_path)?;
 let mut pso2_directory_entries: Vec<_> = pso2_directory_entries.map(|a| a.unwrap()).collect();

 pso2ngs_directory_entries.append(&mut pso2_directory_entries);

 pso2ngs_directory_entries.sort_by(|a, b| {
  let a = a.metadata().unwrap().modified().unwrap();
  let b = b.metadata().unwrap().modified().unwrap();
  b.cmp(&a)
 });

 let latest_entry = pso2ngs_directory_entries
  .iter()
  .find(|a| a.file_name().to_string_lossy().starts_with("ChatLog"))
  .ok_or(NgsLogActionError::ErrorCode(100))?;

 Ok(latest_entry.path())
}

async fn get_latest_chat_log_reader() -> Result<BufReader<DecodeReaderBytes<File, Vec<u8>>>> {
 let latest = get_latest_chat_log_file_path().await?;
 let file = fs::OpenOptions::new()
  .read(true)
  .write(false)
  .create(false)
  .open(latest)?;

 let reader = DecodeReaderBytes::new(file);
 let reader = BufReader::new(reader);
 Ok(reader)
}

async fn get_last_line() -> Result<String> {
 let reader = get_latest_chat_log_reader().await?;
 let lines = reader.lines();

 let line = lines
  .last()
  .ok_or(NgsLogActionError::ErrorCode(101))?
  .map_err(|_| NgsLogActionError::ErrorCode(102))?;

 Ok(line)
}

async fn get_last_datetime() -> Result<DateTime<FixedOffset>> {
 if let Ok(line) = get_last_line().await {
  if let Ok((datetime, _)) = extract_datetime(&line) {
   return Ok(datetime);
  }
 }

 let tz_offset = Local.timestamp(0, 0).offset().fix();
 Ok(Local::now().with_timezone(&tz_offset))
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

async fn get_new_lines(last_datetime: DateTime<FixedOffset>) -> Result<Vec<NgsLog>> {
 let reader = get_latest_chat_log_reader().await?;
 let lines = reader.lines();

 let mut new_lines = Vec::new();

 for line in lines {
  if let Ok(line) = line {
   match extract_datetime(&line) {
    // 過去ログ
    Ok((datetime, _)) if datetime <= last_datetime => (),
    // 新規ログ
    Ok((datetime, tail)) if datetime > last_datetime => {
     let mut tail = tail.split("\t");
     let log_id = tail.next().unwrap().parse()?;
     let channel = NgsLogChannel::from_str(tail.next().unwrap()).unwrap();
     let player_id = tail.next().unwrap().parse()?;
     let name = tail.next().unwrap().to_string();
     let mut body = unescape_double_quote(tail.next().unwrap());
     // 複数行の最初の行
     if body.starts_with(r#"""#) && body.char_indices().nth(1).unwrap().1 != '"' {
      body = body[1..].to_string();
     }
     new_lines.push(NgsLog {
      datetime,
      log_id,
      channel,
      player_id,
      name,
      body,
     })
    }
    // 新規ログまたは新規ログの2行目以降
    _ => {
     if let Some(last_line) = new_lines.last_mut() {
      let line = pre_unescape_double_quote(&line);
      // 新規ログの2行目以降
      if line.chars().last() == Some('"') {
       // 複数行の最後の行( " で終端 )
       let line = finish_unescape_double_quote(&line[..line.len() - 1]);
       (*last_line).body = format!("{}\n{}", last_line.body, line)
      } else {
       // 複数行の途中の行
       (*last_line).body = format!(
        "{}\n{}",
        last_line.body,
        finish_unescape_double_quote(&line)
       )
      }
     } else {
      // 前回検出した最後のログが複数行だった場合
      ()
     }
    }
   }
  }
 }

 Ok(new_lines)
}

fn extract_datetime(line: &str) -> Result<(DateTime<FixedOffset>, String)> {
 let (first_column, tail) = line
  .split_once("\t")
  .ok_or(NgsLogActionError::ErrorCode(200))?;
 let datetime = parse_datetime(first_column)?;
 Ok((datetime, tail.to_string()))
}
