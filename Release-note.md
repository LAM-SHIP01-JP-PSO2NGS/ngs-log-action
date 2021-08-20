[NGS Log Action Wiki]: https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/wiki

## 1.3.0 (2021-08-20)

+ [表示するログの色を好みに応じて変更](https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/wiki/custom)できるようになりました！
+ [ログの取得間隔を変更](https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/wiki/polling_rate)できるようになりました！
+ [取得アイテム集計機能](https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/wiki/item)が実装されました！
  + 拾ったまたはリワードで得たアイテムとメセタのログを扱えるようになりました。
  + すべてまたは特定のアイテムについて取得状況の表示、一定期間での集計の表示を行えます。
  + アイテムの取得数を条件に、一定数取得するごとにアクションを実行できます。
+ 起動時にアプリのバージョンと起動時刻を表示するようになりました。
+ このバージョンのリリースに併せて [NGS Log Action Wiki][] を作成しました！
## 1.2.3 (2021-08-09)

+ [POSTアクションを使うとNGS Log Actionが落ちる場合があるらしい: 投稿者の名前が非ASCII文字を含む場合に100%発生 #3](https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/issues/3) 不具合を修正しました！

## 1.2.2 (2021-08-08)

+ [複数行のログの末尾に " がくっついてしまっている #2](https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/issues/2) 不具合を修正しました！

## 1.2.1 (2021-07-22)

+ [NGS と PSO2 をブロック移動すると最新のログが更新されなくなる状況が発生してしまう #1](https://github.com/LAM-SHIP01-JP-PSO2NGS/ngs-log-action/issues/1) 不具合を修正しました！

## 1.2.0 (2021-07-22)

+ PSO2(1000年前)にも対応しました。NGSとPSO2をブロック移動しても動作するようになります
+ 新機能 `[global]` 設定と表示の仕方の設定機能を追加しました！
  + `[global]` の `show_action_pattern` で `[Action::Show]` などの表示のON/OFFを設定できます
  + `[global]` の `datetime_format` で時刻表示を `01:23` `01:23:45` `01時23分` などお好みの様式に設定できます
  + `[global]` の `show_channel` で  `GUILD` や `PARTY` などの表示のON/OFFを設定できます。OFFでも種別ごとの色分けは有効です
  + `[global]` の `column_separator` でログの名前のパディング(空白文字で桁併せする長さ)を設定できます
  + `[global]` の `channel_padding_width` でチャンネル名のパディング(空白文字で桁併せする長さ)を設定できます
+ 変更
  + デフォルトの `show_action_pattern` 相当の設定が `true` から `false` へ変更されます
  + デフォルトのカラムを分ける文字列が `タブ文字1つ` から `空白文字1つ` に変更されます
  + デフォルトの名前のパディングが `0` から `30` に変更されます
  + デフォルトのチャンネル名のパディングが `0` から `6` に変更されます
+ その他
  + 付属のサンプル設定ファイル `conf.toml` に `1.1.0` および `1.2.0` で追加された設定の例を追記しました

## 1.1.0 (2021-07-20)

新機能 `ignore_names` `ignore_keywords` `ignore_regex` を追加しました！

この設定を使うと…

1. `ignore_names` に設定した特定の名前のチャットログを無視します
2. `ignore_keywords` に設定した特定のキーワードを含むチャットログを無視します
3. `ignore_regex` に設定した正規表現にマッチするチャットログを無視します

となります。具体的には例えば…

```toml
# 設定の具体例<1> 自分の発言ログ『以外』を show = true したい
[[if]]
ignore_names = [ "L,A.M." ]
action = { show = true }
```

```toml
# 設定の具体例<2> "/la sit" と "/la dance" を含むログ『以外』を show = true したい
# (部分一致で動作するので "/la sit" は "/la sitting" や "/la sit2" などなどにヒットします。"/la dance" も同様です。)
# もちろん、コマンドではなく "ばーか" "あーほ" を無視する設定などもできます。
[[if]]
ignore_keywords = [ "/la sit", "/la dance" ]
action = { show = true }
```

```toml
# 設定の具体例<3> 正規表現でコマンド『以外』を show = true したい
# この正規表現の例では「ログの先頭に / がある場合」という意味になります。
# ignore_keywords では「先頭に」などは扱えませんが正規表現だとそういうこともできます。
[[if]]
ignore_regex = "^/"
action = { show = true }
```

## 1.0.0 (2021-07-19)

最初の動作するバージョンができました！

```md
# NGS Log Action

NGS Log Action は [PSO2NGS][] のチャットに連動して「何か」をしてくれるアプリです。

## 具体的にできる事の例

+ `if` 何であれ `=>` 外部ウィンドウでくっきり読める
+ `if` チャットで自分の名前を呼ばれていたら `=>` お好みの通知音を鳴らす
+ `if` `PUBLIC` 誰かがチャット（＝白チャット）で「雷雨」と発言していたら `=>` お好みの通知音を鳴らす
+ `if` `PARTY` または `GROUP` チャットで `/ラッピー/` と発言していたら `=>` お好みのコマンドを実行して何かをする
+ `if` `GUILD` 自分がチャットで「〘緊急警報発令〙ネクス・ヴェラ」と発言したら `=>` Web API `https://example.com/our_guild_sns/api` を叩く
  + 応用するとPSO2NGSでチャットに特定のキーワードを書いたり聞いたりしたら Web API 経由で Discord や Twitter の BOT に何かしてもらう、伝言のように転送して貰うこともできます。
```

