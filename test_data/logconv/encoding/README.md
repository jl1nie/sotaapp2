# 文字コード・改行・末尾のバイトのフィクスチャ

`api/src/handler/logconv/tests/encoding.rs` が読む。`python3 generate.py` で作り直す(標準ライブラリだけ)。
`python3 generate.py --check` で、いまのファイルが生成の結果と一致するかを確かめる。

HAMLOG CSV の列は `コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,`。
自局の参照は備考1、相手の参照は QTH(ASCII)。日本語は名前・QTH・備考のそばに置き、列がずれないことを確かめる。

| ファイル | 文字コード・形 | 内容 |
|---|---|---|
| `sjis_names.csv` | Shift_JIS(cp932)・CRLF | 4行。2バイト目が 0x5C の字(表・能・ソ・十・予・﨑)、半角カナ、cp932 の拡張(①・㈱・髙・﨑・№)、波ダッシュ・全角の `＼` を、名前・QTH・備考1・備考2 に置く |
| `utf8_names.csv` / `utf8_bom_names.csv` | UTF-8(BOM なし・あり)・CRLF | `sjis_names.csv` と同じ内容 |
| `utf16le_bom_names.csv` / `utf16be_bom_names.csv` | UTF-16 LE・BE(BOM つき) | 同じ内容。いまは読める(仕様の確認が要る) |
| `utf16le_nobom_names.csv` | UTF-16 LE(BOM なし) | 同じ内容。いまは全行が誤りになる(仕様の確認が要る) |
| `sjis_error_text.csv` | Shift_JIS | 4行。2・3 行目は、日付の列に日本語(`令和7年10月18日①㈱髙﨑№`、`表能ソ十予/ｱｲｳ`)。誤りの知らせにその文字が出ることで、復号を確かめる |
| `utf8_error_text.csv` / `utf8_bom_error_text.csv` | UTF-8(BOM なし・あり) | 同じ内容 |
| `sjis_utf8_lookalike.csv` | Shift_JIS | 日付の列が Shift_JIS の `縺ｱ`(E3 81 B1)。UTF-8 としても正しいので `ぱ` と読まれる |
| `sjis_invalid_bytes.csv` | Shift_JIS(壊れたバイトあり) | 2行目の名前に 0xFF 0xFE、3行目の名前の終わりに 0x83(2バイト文字の1バイト目だけ)、4行目の行末に 0x81、5行目の日付が `25<FF>10<FF>18`(この行だけが誤り) |
| `utf8_invalid_byte.csv` | UTF-8(0xFF が1つ) | ファイル全体が Shift_JIS として読まれる(日本語は化けるが、ASCII の列は無事) |
| `newline_crlf.csv` / `newline_lf.csv` / `newline_cr.csv` | ASCII | 2行。改行が CRLF・LF・CR だけ |
| `newline_none_at_end.csv` | ASCII | 2行。最後の行に改行がない |
| `newline_mixed.csv` | ASCII | 3行。CRLF と LF が混ざる |
| `sub_eof_after_newline.csv` | ASCII | 2行のあと、最後の改行の後ろに 0x1A(古い DOS の EOF) |
| `sub_eof_no_newline.csv` | ASCII | 2行目の行末(改行なし)に 0x1A が付く |
| `ascii_only.csv` | ASCII | 2行 |
