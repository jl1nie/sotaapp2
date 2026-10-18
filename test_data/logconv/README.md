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
