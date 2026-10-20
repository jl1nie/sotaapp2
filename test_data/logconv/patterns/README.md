# logconv のパターン網羅フィクスチャ

デプロイ後の確認と、変換のテストに使う。入力(CSV・ADIF)と、旧版の出力(正解)と、期待の一覧(`cases.json`)から成る。

## 確認のしかた

    python3 verify.py https://sotaapp2.fly.dev        # 本番のバックエンド
    python3 verify.py https://logconv2.sotalive.net   # 画面と同じ入口
    python3 verify.py URL -k pota/                    # 一部のケースだけ

標準ライブラリだけで動く。差があれば、ケース名と最初に違う行を出して、終了コード 1。
同じケースを、Rust のテスト(`api/src/handler/logconv.rs` の `test_patterns_*`)も、ハンドラーを直接呼んで確かめる。

## 入力(`*.csv` / `input.adi`)

HAMLOG CSV の列は `コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,`。
自局の参照は備考1(`references_my_rmks2.csv` は備考2)、相手の参照は QTH と備考2(同じ内容。画面の指定に合わせて、どちらでも読めるように)。

| ファイル | パターン |
|---|---|
| `bands_modes.csv` | 全バンド(135kHz〜10GHz)× 全モード(CW・SSB・FM・AM・RTTY・FT8・FT4・PSK31・JT・DV 系・不明なモード)、RST が1桁・SNR・空、周波数が `a/b` |
| `references.csv` | 自局・相手の参照の組み合わせ: 山のみ / 山+公園+WWFF / S2S / P2P / W2W / S2S+P2P+W2W / 相手 2-fer・3-fer / 自局 2-fer・3-fer(n×m)/ 公園だけ・WWFF だけ / 山の移動 / 小文字 / 座標つき / 区切り(空白2つ・カンマ)/ 参照なし |
| `references_my_rmks2.csv` | 同じ内容。自局が備考2、相手が備考1(画面の「Remarks2で指定する」) |
| `multiday_tz.csv` | UTC・JST(`J`)・Z、UTC の日付をまたぐ、4桁の年・2桁の年(99→1999)、年またぎ、うるう日。旧版と比べるため、UTC の時刻順 |
| `unordered.csv` | 日付が行き来するログ(旧版は、先の交信が消えていた) |
| `errors.csv` | 日付・時刻の書式、存在しない日時、バンドにない周波数・数字でない周波数、列が足りない行、空行 |
| `sjis.csv` | Shift-JIS(日本語版 HAMLOG)。名前・QTH・備考に日本語、2バイト目が `0x5C` の字(表・能・ソ) |
| `ios.csv` | HamLog iOS 形式(1行目が見出し)。JST で UTC の日付をまたぐ交信を含む |
| `input.adi` | ADIF を入力にする(山・公園・FREQ・6桁の時刻・SUBMODE・コメント) |

作り直すとき: `python3 generate.py`(`sjis.csv` は cp932 で書く)。

## 正解と期待

- `expected/`: 旧版(HamLogConv の Python)の出力。`HAMLOGCONV_DIR=~/src/HamLogConv python3 legacy_snapshot.py` で作る(adif-io と requests が要る。公園の所在地の取得は外してある)。ADIF は `check.json`(CSV読込の結果)も。
  チェイサーは、旧版が全日付を1ファイルにするので、日付ごとに分け直してある(新版は日付ごと)。
- `cases.json`: ケースの一覧。`expect` があるものは、旧版と比べず、新版の決めた動作を確かめる(誤りのある行・順不同のログ・公園の所在地)。
  - 誤りのあるログは、ADIF を生成しない(400)。誤りの行は `sota000000.csv` に集める。
  - `live_only` は、本番の公園データが要るもの(`MY_STATE`)で、`verify.py` だけが確かめる。

## encoding / errors / options のケース

`cases.json` の `encoding/…`・`errors/…`・`options/…` は、`../encoding/`・`../errors/` のフィクスチャ(`file` に相対パスで書く)で、
文字コード(Shift_JIS・UTF-8・BOM)、例外系(空・見出しだけ・引用符が閉じない・日付の境目・全行が誤り)、画面の `options` の名前を、本番で確かめる。
`expect.contains` は `verify.py` だけが確かめる(日本語が誤りの知らせに正しく出ること、BOM がコールサインに混ざらないこと)。
