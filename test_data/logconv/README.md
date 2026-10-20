# logconv のテストデータ

HAMLOG CSV(`コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,`)と、WSPRnet のスポットの例。
自局(`JA1ZZZ/1`)の参照は備考1、相手局の参照は QTH(`hamlog_nfer_rmks2.csv` は備考2)の列にある。

| ファイル | 内容 |
|---|---|
| `hamlog_nfer.csv` | 通常・S2S・P2P・W2W、相手 2〜3-fer、自局 2〜3-fer(n×m)の10交信 |
| `hamlog_nfer_rmks2.csv` | 同じ内容。相手の参照が備考2の列(SOTA アクティベーターの画面は備考2で指定する) |
| `hamlog_jst_errors.csv` | JST(`J`)で UTC の日付をまたぐ交信、日付が戻る行、存在しない時刻、不明な周波数、参照のない行 |
| `wsprnet_spots_20261018.txt` | WSPRnet の画面からそのままコピーした50スポット。見出し・件数・フッターなどの余計な行を含む(13項目の行だけが読まれる) |

テストは `service/src/implement/logconv/converter.rs` と `service/src/implement/wspr_service.rs`。

## legacy/ — 旧版(HamLogConv の Python)の出力
`hamlog_nfer.csv` を旧版の `convutil.py` に通して得た出力(改行は LF にそろえてある)。新版の出力が旧版と同じになることのテストの正解。
- `legacy/sota_a/`: SOTA Activator(`hamlog_nfer_rmks2.csv`、`JA1ZZZ/1`、自局=備考1、相手=備考2)。CSV・S2S・ADIF
- `legacy/sota_c/`: SOTA Chaser(`JA1ZZZ`、相手=QTH)。`sota…`(山のある交信)と `other…`(ない交信)
- `legacy/pota/`: POTA/WWFF/SOTA の ADIF(`JA1ZZZ/1`、運用者 `JA1ZZZ`)。MY_STATE は、公園の所在地の取得(ネットワーク)を外してある

## patterns/ — パターンを網羅したフィクスチャ
デプロイ後の確認(`patterns/verify.py`)と、変換のテストに使う。詳しくは `patterns/README.md`。

## encoding/ — 文字コード・改行・末尾のバイト
Shift_JIS(cp932)・UTF-8(BOM)・UTF-16・壊れたバイト・改行・末尾の 0x1A。`encoding/generate.py` で作る。一覧は `encoding/README.md`(`api/src/handler/logconv/tests/encoding.rs`)。

## errors/ — 誤った入力
空・見出しだけ・画像・引用符が閉じない・壊れた ADIF など。`errors/generate.py` で作る。一覧は `errors/README.md`(`api/src/handler/logconv/tests/errors.rs`)。

## fle/ — FLE Online が出した HAMLOG の CSV(e2e)
`名前.fle` が FLE のログ、`名前/` が FLE Online の ZIP の中身(HAMLOG の CSV は Shift_JIS・CRLF。ほかに FLE が出す ADIF・SOTA の CSV)。
HAMLOG の CSV を logconv の画面と同じ指定で変換し、FLE が自分で出した ADIF・SOTA の CSV と、交信ごとに同じになることを確かめる
(`api/src/handler/logconv/tests/fle.rs`)。

| ファイル | 内容 |
|---|---|
| `pota_nfer.fle` | 自局 2-fer(+WWFF)× 相手 0〜3-fer、全バンド(135kHz〜10GHz)、日本語の名前・備考(表・能・ソ) |
| `sota_s2s.fle` | S2S・S2S+P2P・座標つき。JST で書いて、UTC の日付をまたぐ |
| `sota_pota.fle` | 山と公園の同時運用。相手は S2S・P2P・S2S+P2P |
| `chaser.fle` | チェイサー(自局の参照なし)。山との交信と、そうでない交信 |
| `pota_move.fle` | 途中で運用先(山・公園)を変える |

FLE の HAMLOG の CSV は、自局の参照(QSO ごとの運用先)を備考1に、相手の参照を QTH の列に書く。画面では、運用先を
「Remarks1で指定する」(か、入力)にし、相手を「QTHで指定する」にする。途中で運用先を変えたログは、Remarks1 でなければ分けられない。

作り直すとき(sotalive-monorepo で):

    cargo run -p fle-core --example logconv_fixtures -- ~/src/sotaapp2/test_data/logconv/fle
