# fireauth (vendored)

crates.io から削除された `firebase-auth-sdk 0.1.0` の代わりに、同じ構成・同じ API を持つ
`the-robot/fireauth` を取り込んだもの。

- 取得元: https://github.com/the-robot/fireauth
- コミット: `78df85cf83ba5d87602395d54dfe2bc1cb8f4e15`(crate バージョン 0.1.5)
- ライセンス: WTFPL(`LICENSE`)
- 取得日: 2026-10-17
- 取り込んだファイル: `Cargo.toml` `LICENSE` `README.md` `src/**`(`logo.png` は除外)

## 元のコードからの変更(最小限の補強のみ)
1. `get_user_info`: `body.users[0]` の添字参照を、空のとき `Error::User("USER_NOT_FOUND")` を返す形に変更(パニック回避)
2. `User` / `SignIn Response`: 利用側が使わないフィールドに `#[serde(default)]` を付与
   (パスワード以外で登録したアカウントなど、レスポンスにフィールドが無い場合でも解析に失敗しないように)
   `local_id` と `id_token` は必須のまま
3. `cargo fmt --all` がパス依存も対象にするため、`rustfmt` で整形(体裁のみ。`mod` の並びも含む)

sotaapp2 が使うのは `FireAuth::new` / `get_user_info`(`local_id`)/ `sign_in_email`(`id_token`)のみ。
