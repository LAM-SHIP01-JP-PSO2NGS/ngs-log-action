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

