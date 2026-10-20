# 誤った入力のフィクスチャ

`api/src/handler/logconv/tests/errors.rs` が読む。`python3 generate.py` で作り直す(標準ライブラリだけ)。
`python3 generate.py --check` で、いまのファイルが生成の結果と一致するかを確かめる。
(ほかの誤った入力は、テストの中で組み立てる。日時・改行・multipart・1万行など)

| ファイル | 内容 |
|---|---|
| `empty.csv` | 0 バイト |
| `blank_lines.csv` | 空行・空白だけの行・タブだけの行 |
| `header_only_ios.csv` | HamLog iOS の見出しの行だけ |
| `png_like.bin` | 1x1 の PNG(画像をログとして送ったとき) |
| `unclosed_quote.csv` | 2行目の引用符が閉じない(3行目はその項目に飲み込まれる) |
| `too_many_columns.csv` | 15列より多い行 |
| `adif_sjis_comment_len.adi` | Shift_JIS の ADIF。`<COMMENT:4>山頂`(長さが Shift_JIS のバイト数)。UTF-8 にすると長さが文字の途中にあたる |
| `adif_fullwidth_date.adi` | ADIF の `QSO_DATE` が全角数字 |
| `adif_fullwidth_time.adi` | ADIF の `TIME_ON` が全角数字 |
| `adif_header_special_char.adi` | ADIF のヘッダーに `ŉ`(大文字にすると1バイト増える字) |
| `adif_nonexistent_datetime.adi` | ADIF の日付 13月45日・時刻 99:99 |
| `adif_no_eor.adi` | ADIF の最後の記録に `<EOR>` がない |
| `dates_calendar.csv` | 8行。2・3・8 行目が存在しない日付(平年の 2/29・2/30・4/31)。4 行目は JST で 3/1 08:00(UTC ではうるう日 2/29 23:00)、6 行目は JST の年始 08:30(UTC では前年末 23:30) |
| `all_bad.csv` | 3行すべてが存在しない時刻(99:99) |
