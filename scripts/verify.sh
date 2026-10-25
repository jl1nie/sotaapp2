#!/usr/bin/env bash
# 「通ったか」を、読み違えようのない 1 行で答える(docs/agent-workflow.md 8.1)。
#
#   scripts/verify.sh                全部の段階を走らせ、最初の失敗で止まる
#   scripts/verify.sh --keep-going   失敗しても全部の段階を走らせる
#   scripts/verify.sh --quick        clean を飛ばす(手元で直しているあいだ)
#
# 段階(CI の .github/workflows/ci.yml と同じコマンド):
#   fmt       cargo fmt --all -- --check
#   clippy    cargo clippy --all-targets --all-features -- -D warnings
#   test      cargo nextest run --workspace --test-threads=1(手元の .env.test があれば読む。Makefile.toml の test と同じ)
#   admin     admin/ で npm run check(svelte-check)と vitest run。**npm run build はここではしない**
#             (build は、追跡している static/ を書き換える。clean の段階の中でだけ走らせる)
#   clean     コミットした HEAD を、きれいなチェックアウト(git worktree。未追跡・無視のファイルなし)で、
#             SQLX_OFFLINE=true・.env.test なし(CI と同じ)の cargo nextest と、admin/ の npm ci → check → vitest →
#             build。CI が見るもの。**先にコミットする**(HEAD を試すので、作業ツリーの変更は見ない)。
#
# 最後の行は `VERIFY PASS` か `VERIFY FAIL: <段階>[, <段階>]`、終了コードは PASS のときだけ 0。
# 結果はこの行だけで読む。ラッパーなしで直接走らせる(`run_in_background` に `; echo` を足さない):
# ラッパーの終了コードはラッパーのもので、スクリプトの失敗を隠す。段階ごとのログは $VERIFY_DIR
# (既定 target/verify)に残り、失敗した段階の最後の行を出す。
#
# `clean` がある理由: `.env.test` などの .env.* は git に入れない。手元にだけあるファイルに頼って通っても、CI では
# 通らない。admin の build も、きれいなチェックアウトで試す(手元の build は追跡対象の static/ を書き換える)。
# CI には admin の型検査・vitest のジョブがない(Docker のビルドの中の npm run build だけ)ので、ここが唯一の確認になる。
set -uo pipefail

cd "$(git rev-parse --show-toplevel)"

KEEP_GOING=0
QUICK=0
for arg in "$@"; do
  case "$arg" in
    --keep-going) KEEP_GOING=1 ;;
    --quick) QUICK=1 ;;
    -h | --help)
      sed -n '2,/^set -uo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "verify.sh: 知らないオプション '$arg'" >&2
      exit 2
      ;;
  esac
done

VERIFY_DIR="${VERIFY_DIR:-target/verify}"
mkdir -p "$VERIFY_DIR"
failed=()
n=0

strip_ansi() { sed 's/\x1b\[[0-9;]*m//g'; }

# stage <名前> <コマンド...>: 出力をログに取り、1 行で結果を出す。
stage() {
  local name="$1"
  shift
  n=$((n + 1))
  local log="$VERIFY_DIR/$(printf '%02d' "$n")-$name.log"
  local start=$SECONDS
  "$@" >"$log" 2>&1
  local status=$?
  local took=$((SECONDS - start))
  # nextest は `Summary [...] N tests run: N passed, N failed`、vitest は `Tests  N passed` を出す。
  local counts
  counts=$(strip_ansi <"$log" | grep -E '^ *(Summary|Tests +[0-9])' | tail -n 1 | sed -E 's/^ +//; s/ +/ /g')
  local failed_tests=0
  if [ -n "$counts" ] && echo "$counts" | grep -qE '[1-9][0-9]* failed'; then
    failed_tests=1
  fi
  if [ "$status" -eq 0 ] && [ "$failed_tests" -eq 0 ]; then
    echo "PASS  $name (${took}s)${counts:+ — $counts}"
    return 0
  fi
  echo "FAIL  $name (${took}s, exit $status)${counts:+ — $counts}   log: $log"
  echo "  ---- $log の最後 ----"
  tail -n 15 "$log" | strip_ansi | sed 's/^/  | /'
  failed+=("$name")
  return 1
}

# --keep-going でなければ、最初の失敗で止まる。
run() {
  stage "$@" || { [ "$KEEP_GOING" = 1 ] || finish; }
}

finish() {
  if [ "${#failed[@]}" -eq 0 ]; then
    echo "VERIFY PASS"
    exit 0
  fi
  local IFS=,
  echo "VERIFY FAIL: ${failed[*]}" | sed 's/,/, /g'
  exit 1
}

# 手元のテスト: .env.test があれば読む(Makefile.toml の test と同じ)
local_tests() {
  set -a
  # shellcheck disable=SC1091
  [ -f .env.test ] && . ./.env.test
  set +a
  SQLX_OFFLINE=true cargo nextest run --workspace --status-level fail --test-threads=1
}

admin_checks() {
  (cd admin && npm run check && npx vitest run)
}

# コミットした HEAD の、きれいなチェックアウトで、CI と同じ確認をする。cargo の target は共有する
# (外部のクレートは作り直さない)。
clean_checkout() {
  local root dir
  root="$(pwd)"
  dir="$(mktemp -d "${TMPDIR:-/tmp}/sotaapp2-verify-clean.XXXXXX")" || return 1
  git worktree add --detach "$dir" HEAD >/dev/null || return 1
  (
    cd "$dir" &&
      env -u DATABASE_URL SQLX_OFFLINE=true CARGO_TARGET_DIR="$root/target" \
        cargo nextest run --workspace --status-level fail --test-threads=1 &&
      cd admin && npm ci && npm run check && npx vitest run && npm run build
  )
  local status=$?
  git worktree remove --force "$dir" >/dev/null 2>&1
  rm -rf "$dir"
  return $status
}

run fmt cargo fmt --all -- --check
run clippy cargo clippy --all-targets --all-features -- -D warnings
run test local_tests
run admin admin_checks
if [ "$QUICK" = 0 ]; then
  if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "NOTE  clean: 作業ツリーにコミットしていない変更がある。この段階は HEAD だけを試す"
  fi
  run clean clean_checkout
else
  echo "SKIP  clean (--quick。CI が見るきれいなチェックアウトは試していない)"
fi
finish
