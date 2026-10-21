# logconv のテスト計画(本番展開の前に)

LogConverter の新版(画面: sotalive-monorepo の `apps/logconv`、変換: このリポジトリの `logconv`)を、
logconv.sotalive.net の本番に出す前に、変換の正しさと、誤った入力への振る舞いを確かめる。

## 方針

- まず既存のテストを読み、すでに確かめていることは足さない。足りないところだけを足す。
- テストのために本体のコードを変えない。テストで不具合が見つかったら、そのテストは
  `#[ignore = "不具合: …"]`(Rust)/ `it.fails` か `it.todo`(Vitest)で残し、一覧(下の「見つかった不具合」)に書く。直すかどうかは別に決める。
- 振る舞いが決まっていない入力(例: UTF-16 のファイル)は、いまの振る舞いをテストで固定し、「仕様の確認が要る」として一覧に書く。
- フィクスチャは `test_data/logconv/` に置く。作り方(生成のスクリプトか手順)も残す。

## 既存のテスト(2026-10-21 時点)

| 層 | 場所 | 確かめていること |
|---|---|---|
| 読み込み | `service/src/implement/logconv/hamlog.rs` | HAMLOG・iOS の行、JST→UTC、2桁の年、存在しない時刻、列の不足 |
| 参照 | `service/src/implement/logconv/reference.rs` | 公園の 4〜5桁、複数の参照、座標、参照のない文 |
| 変換 | `converter/tests.rs`、`converter/tests/matrix.rs` | 旧版との一致(sota_a・sota_c・pota・iOS)、POTA の自局 n×相手 m-fer、S2S、列の組み合わせの一部 |
| API | `api/src/handler/logconv/tests.rs` | multipart、未知の形式は 400、誤りのあるログは ADIF を作らない、ADIF の入力、patterns の 29 ケース |
| e2e | `api/src/handler/logconv/tests/fle.rs` | FLE Online の HAMLOG CSV(Shift_JIS・CRLF)を変換して、FLE 自身の出力と一致 |
| 画面 | monorepo `apps/logconv/src/**/*.test.tsx` | 各タブの送信と表示の一部、WSPR の読み取り |
| 本番 | `test_data/logconv/patterns/verify.py` | 配備したバックエンドで patterns のケース |

## 足す範囲

### A. SOTA・POTA の扱いの網羅(service)

1. **形式 × 自局の列 × 相手の列の全組み合わせ**: 形式 3(`sota_a`・`sota_c`・`pota`)× 自局 3(`form`・`rmks1`・`rmks2`)× 相手 4(`none`・`qth`・`rmks1`・`rmks2`)。
   自局と相手が同じ列を指す組み合わせ(例: 自局 `rmks1` × 相手 `rmks1`)の振る舞いも固定する。
2. **自局の参照の種類**: 山だけ / 公園だけ / WWFF だけ / 山+公園 / 公園 2〜3-fer / 山の移動(途中で変わる)/ 自局の参照のない行の混在。
3. **相手の参照**: S2S / P2P / W2W / 相手の n-fer / 相手の参照と自由な文が混ざった欄 / 座標つき / 小文字 / 全角の英数字・全角の空白・全角のハイフン(日本語の入力で起きる)。
4. **出力の中身**: SOTA の CSV(V2 の列、日付ごとのファイル、S2S のファイル)、ADIF(`MY_SOTA_REF`・`MY_POTA_REF`・`MY_SIG`・`SIG_INFO`・`POTA_REF`・`SOTA_REF`・`OPERATOR`・`STATION_CALLSIGN`)を、ケースごとに期待の値で確かめる。

### B. 画面で指定した参照とコールサイン(service + api)

1. `Summit`: 正しい形 / 小文字 / 前後の空白 / `JA/TK-001` 以外の協会 / 誤った形 / 空(`sota_a` で `form` のとき)。
2. `Park`: 1つ / 空白・カンマ区切りの複数 / POTA と JAFF の混在 / 4桁・5桁 / 誤った形 / 空。
3. 指定と列の優先: `form` のとき列の参照を無視するか。列を指定したとき `Summit`・`Park` を無視するか。
4. コールサイン: `SOTAActivator`・`POTAActivator`・`POTAOperator` の空・小文字・`/1` `/P` つき・誤った形。運用者を省いたときの扱い。

### C. 例外系(api)

1. 入力ファイル: 空 / 見出しだけ / 空行だけ / バイナリ(画像)/ 拡張子とちがう中身 / 列が多すぎる行 / 引用符が閉じない行 / 1万行(時間とメモリの目安を記録)/ 上限(8MB。logconv だけの上限)を超える。
2. multipart: `file` がない / `options` がない / `options` が JSON でない / 未知の項目 / 未知の `format`・`command`。
3. 応答: どのケースでも、500 やパニックにならず、利用者に読めるメッセージを返す。誤りの行番号が元のファイルの行と合う(CRLF・LF・CR だけ・空行を含む)。
4. 一部の行だけ誤り: `pota` は生成しない(400)、`sota_a`・`sota_c` は誤りの行を `sota000000.csv` に集める、という決まりを全形式で確かめる。全行が誤りのとき。
5. 日時: 日付が戻る・同じ交信の重複・未来の日付・年またぎ・うるう日・存在しない日付(2/30)。

### D. 文字コード(api の `decode_text` と、変換の出力)

1. **Shift_JIS(cp932)**: 日本語版 HAMLOG の CSV。2バイト目が `0x5C` の字(表・能・ソ・十・予)、半角カナ、cp932 の拡張(①・㈱・髙・﨑・№)、名前・QTH・備考のどこにあっても列がずれない。
2. **UTF-8**: BOM あり・なし。BOM が最初のコールサインに混ざらない。
3. **判定の境目**: ASCII だけのファイル、UTF-8 としても読める Shift_JIS の並び、壊れたバイトを含むファイル(置き換え文字になる行と、ほかの行が読めること)。
4. **UTF-16**(BOM つき): いまの振る舞いを固定し、仕様の確認に挙げる。
5. **改行と終わり**: CRLF・LF・CR だけ、最後の行に改行がない、末尾の `0x1A`(古い DOS の EOF)。
6. **出力**: ADIF の項目の長さ(`<NAME:n>`)がバイト数で正しいか(日本語の名前・備考を出す場合)、または日本語を出さない決まりか。SOTA の CSV に日本語が入る列と、その文字コード。ZIP の中のファイル名。

### E. 画面(monorepo `apps/logconv`、Vitest)

1. 各タブで、画面の選択(自局・相手の列、`Summit`・`Park`、コールサイン)が、API の `options` に正しく対応する(全選択肢)。
2. ファイルはそのままのバイトで送る(画面で文字コードを変えない)。
3. API の 400(誤りの一覧)・500・通信の失敗・時間切れで、利用者に読めるメッセージが出て、画面が固まらない。
4. ダウンロードのファイル名、保存した設定(Zustand の persist)の復元と、古い形の保存データの読み込み。

### F. 本番の確認(展開の手順に入れる)

1. `verify.py` に、A〜D で足したフィクスチャのうち本番で確かめる価値のあるものを `cases.json` で足す。
2. 展開の前後に `python3 verify.py https://logconv2.sotalive.net` を流す。

## 分担

| 担当 | リポジトリ | 範囲 | 触るファイル |
|---|---|---|---|
| 1 | sotaapp2 | A・B(service)、D-6 | `service/src/implement/logconv/converter/tests/` の下に新しいファイル(`options_matrix.rs`、`references.rs`、`output_encoding.rs` など) |
| 2 | sotaapp2 | B(api)・C・D-1〜5、F | `api/src/handler/logconv/tests/` の下に新しいファイル、`test_data/logconv/encoding/`・`test_data/logconv/errors/`、`patterns/cases.json` |
| 3 | sotalive-monorepo | E | `apps/logconv/src/**/*.test.tsx` |

## 見つかった不具合・仕様の確認が要る点(2026-10-21 の実行)

テストは `#[ignore = "不具合: …"]`(画面は `it.fails`)で残してある。直したら外す。

### 不具合(sotaapp2)
| # | 内容 | テスト |
|---|---|---|
| 1 | ADIF: Shift_JIS の ADIF で、長さが Shift_JIS のバイト数の項目(`<COMMENT:4>山頂`)があるとパニック。**直した。ADIF を元の文字コードのバイト列のまま読み、値ごとに文字列にする** | `errors::adif_length_inside_a_multibyte_char_does_not_panic` |
| 2 | ADIF: `QSO_DATE`・`TIME_ON` が全角数字だとパニック。**直した。全角などの日付・時刻は誤りの記録(400 と誤りの一覧)** | `errors::adif_non_ascii_date_or_time_does_not_panic` |
| 3 | ADIF: ヘッダーに大文字にすると長さが変わる字(`ŉ`)があると、最初の記録が欠ける。**直した。`<EOH>` を、バイト列の中で大文字小文字を区別せず探す** | `errors::adif_header_with_special_characters_keeps_the_first_record` |
| 4 | ADIF: 存在しない日時(13月45日、99:99)を検査しない。**直した。存在しない日時は `日時不正:…` の誤り(HAMLOG CSV と同じ)** | `errors::adif_nonexistent_datetime_is_an_error_like_csv` |
| 5 | ADIF: 最後の記録に `<EOR>` がない・`CALL` がない記録が、黙って消える。**直した。`<EOR>` のない最後の記録も読む。`CALL` のない記録は、記録の番号つきの誤り** | `errors::adif_record_without_eor_or_call_is_not_dropped_silently` |
| 6 | 改行の後ろの `0x1A`(DOS の EOF)が誤りの行になり、POTA は ADIF を作れない。**直した。末尾の 0x1A(と、それに続く空白)を読む前に除く** | `encoding::sub_eof_after_the_last_newline_is_ignored` |
| 7 | 改行が CR だけのファイルで、誤りの行番号が常に 1。**直した。CRLF・LF・CR を行の区切りとして数える** | `errors` の CR の行番号 |
| 8 | 行番号を毎行、先頭から数え直す(行数の2乗)。release で1万行が約 1.3 秒/回。**直した。行の開始位置を先に作って二分探索(debug で1万行が約 1 秒)。`ten_thousand_rows_are_converted` は計測用に ignore のまま** | `errors::ten_thousand_rows_are_converted`(計測) |
| 9 | 全角の数字を含む参照(`JA-０１２３`)を読み、そのまま ADIF に出す。**直した。全角を半角にしてから参照を読み、参照の正規表現の数字は ASCII だけ** | `references::fullwidth_digits_after_ascii_prefix_do_not_leak_into_adif` |
| 10 | 画面で入れた `Summit` を大文字にせず、空白も除かない(`Park` はそろえる)。**直した。`Summit` は全角を半角にし、前後の空白を除いて大文字にする** | `options_matrix::summit_in_the_form_is_trimmed_and_uppercased` |
| 11 | コールサインの `\`・`..` が ZIP の中の名前に残る(`/` だけ置き換え)。**直した。`/`・`\`・Windows で使えない字・制御文字を `-` にする** | `output_encoding::zip_names_never_contain_backslash_or_dot_dot_components` |

### 不具合(画面 apps/logconv)
| # | 内容 |
|---|---|
| 12 | 参照が1つもないときの 200 の JSON(`No SOTA references found`)を、ZIP として保存する |
| 13 | 時間切れがない(応答がないと、ボタンが押せないまま) |
| 14 | SOTA Activator・Chaser は、400 の理由(`error_log`)を出さない |
| 15 | コールサインの前後の空白を除かない(ファイル名・API に空白が入る) |
| 16 | 保存した設定のコールサインが null だと、押しても何も起きない |
| 17 | 保存した設定の選択肢が知らない値のとき、既定に直さない |
| 18 | `localStorage` に書けないと、入力のたびに例外 |

### 仕様の確認が要る点(2026-10-21 に決めて、直した)
- 全角の英数字・記号(U+FF01〜U+FF5E)は半角に、全角の空白(U+3000)は半角の空白にしてから、参照(列と、画面の Summit・Park)とコールサインを読む。日本語の名前などほかの列は変えない。
- コールサイン(SOTAActivator・POTAActivator・POTAOperator・callsign)は、前後の空白を除いて大文字にそろえ、すべての出力(CSV の自局、STATION_CALLSIGN、OPERATOR、ファイル名)で同じにする。
- 必要なコールサインが空なら 400(sota_a は運用局、sota_c は自局、pota は運用局。`/logconv/pota` の確認は断らない)。
- SOTA で読める交信が1つもない(全行が誤り・バイナリなど)ときは、200 の ZIP ではなく 400 と誤りの一覧(`error_log`)。一部だけが誤りなら、これまでどおり ZIP に `sota000000.csv`。
- `sota000000.csv` は UTF-8 の BOM つき。
- 8MB を超える入力は、413 と「ファイルが大きすぎます(上限 8MB)」(本番のマシンのメモリに合わせた上限。v2 全体は 64MB)。
- 変えない: UTF-16(BOM つきは読む、なしは誤り)、重複・未来の日付は通す、ADIF は `MY_SIG`・`SIG_INFO` で出す(`MY_POTA_REF`・`POTA_REF` は使わない。旧版と同じ)、`Summit`・`Park` の形は検査しない、通信の失敗は、ブラウザのメッセージをそのまま出す(画面の担当)。
