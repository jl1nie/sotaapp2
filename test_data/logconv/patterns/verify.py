#!/usr/bin/env python3
"""デプロイしたサーバーに、パターンのフィクスチャを送って、期待どおりの出力が返ることを確かめる。

    python3 verify.py https://sotaapp2.fly.dev            # 本番のバックエンド
    python3 verify.py https://logconv2.sotalive.net       # 画面と同じ入口(Cloudflare の中継)
    python3 verify.py URL -k pota/                        # 名前に pota/ を含むケースだけ

標準ライブラリだけで動く。終了コードは、差があれば 1。
- expected/ があるケース: 旧版(HamLogConv)の出力と、ファイルの組・中身が同じ
- expect があるケース(誤りのある行など): cases.json の期待(ファイル・行数・誤りの行番号)どおり
- live_only のケース: 本番のデータが要るもの(公園の所在地 MY_STATE)。Rust のテストでは確かめない
"""
import argparse
import io
import json
import os
import sys
import urllib.error
import urllib.request
import uuid
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))


def multipart(file_bytes, options):
    boundary = uuid.uuid4().hex
    body = (f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="log.csv"\r\n\r\n').encode()
    body += file_bytes
    body += (f'\r\n--{boundary}\r\nContent-Disposition: form-data; name="options"\r\n\r\n{options}\r\n--{boundary}--\r\n').encode()
    return body, f'multipart/form-data; boundary={boundary}'


def post(base, path, file_bytes, options):
    body, ctype = multipart(file_bytes, json.dumps(options, ensure_ascii=False))
    req = urllib.request.Request(base.rstrip('/') + path, data=body, headers={'Content-Type': ctype, 'User-Agent': 'logconv-verify/1.0'}, method='POST')  # 既定の Python の UA は、Cloudflare が 403 で拒否する
    try:
        with urllib.request.urlopen(req, timeout=60) as res:
            return res.status, res.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def unzip(data):
    z = zipfile.ZipFile(io.BytesIO(data))
    return {n: z.read(n).decode('utf-8').replace('\r\n', '\n') for n in sorted(z.namelist())}


def read_expected(name):
    d = os.path.join(HERE, 'expected', name)
    files = {}
    for n in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        if n != 'check.json':
            files[n] = open(os.path.join(d, n), encoding='utf-8').read()
    return files


def error_lines(resp):
    log = resp.get('error_log') or resp.get('errorlog') or []
    if isinstance(log, str):
        log = log.splitlines()
    out = []
    for line in log:
        head = line.split('行')[0]
        if head.isdigit():
            out.append(int(head))
    return out


def check_case(base, case):
    """差があれば、その説明のリストを返す"""
    problems = []
    name = case['name']
    file_bytes = open(os.path.join(HERE, case['file']), 'rb').read()
    options = {**case['options'], 'format': case['format']}
    expect = case.get('expect')

    if case.get('endpoint') == 'check':
        status, body = post(base, '/api/v2/logconv/pota', file_bytes, options)
        resp = json.loads(body or b'{}')
        if 'status' in expect and resp.get('status') != expect['status']:
            problems.append(f"status {resp.get('status')}(期待 {expect['status']})")
        if 'error_lines' in expect and error_lines(resp) != expect['error_lines']:
            problems.append(f"誤りの行 {error_lines(resp)}(期待 {expect['error_lines']})")
        return problems

    status, body = post(base, '/api/v2/logconv/hamlog', file_bytes, options)
    if expect and 'http' in expect:
        if status != expect['http']:
            return [f'HTTP {status}(期待 {expect["http"]})']
        if 'error_lines' in expect and error_lines(json.loads(body)) != expect['error_lines']:
            problems.append(f"誤りの行 {error_lines(json.loads(body))}(期待 {expect['error_lines']})")
        return problems
    if status != 200:
        return [f'HTTP {status}: {body[:120]!r}']
    got = unzip(body)

    if expect:  # 新版の決めた動作
        if 'files' in expect and list(got) != expect['files']:
            problems.append(f"ファイル {list(got)}(期待 {expect['files']})")
        for fn, n in expect.get('lines', {}).items():
            c = len(got.get(fn, '').splitlines())
            if c != n:
                problems.append(f'{fn}: {c} 行(期待 {n})')
        for fn, n in expect.get('records', {}).items():
            c = got.get(fn, '').count('<EOR>')
            if c != n:
                problems.append(f'{fn}: {c} 記録(期待 {n})')
        for fn, text in expect.get('contains', {}).items():
            if text not in got.get(fn, ''):
                problems.append(f'{fn}: 「{text}」がない')
        return problems

    # 旧版の出力との一致
    want = read_expected(name)
    if list(got) != list(want):
        return [f'ファイルの組が違う\n    新: {list(got)}\n    旧: {list(want)}']
    for fn, w in want.items():
        g = got[fn]
        if g != w:
            gl, wl = g.splitlines(), w.splitlines()
            i = next((i for i, (a, b) in enumerate(zip(gl, wl)) if a != b), min(len(gl), len(wl)))
            problems.append(f"{fn}: {i + 1}行目から違う\n    新: {gl[i] if i < len(gl) else '(なし)'}\n    旧: {wl[i] if i < len(wl) else '(なし)'}")

    # ADIF は、確認(CSV読込)の結果も旧版と同じ
    check_path = os.path.join(HERE, 'expected', name, 'check.json')
    if os.path.exists(check_path):
        old = json.load(open(check_path, encoding='utf-8'))
        status, body = post(base, '/api/v2/logconv/pota', file_bytes, {**options, 'command': 'ADIFCSVCheck'})
        new = json.loads(body)
        if new.get('status') != old['status']:
            problems.append(f"確認の status {new.get('status')}(旧 {old['status']})")
        if new.get('log_text', []) != old['logtext']:
            problems.append('確認の表(log_text)が違う')
    return problems


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('base', help='例: https://sotaapp2.fly.dev')
    ap.add_argument('-k', default='', help='この文字列を名前に含むケースだけ')
    args = ap.parse_args()

    cases = json.load(open(os.path.join(HERE, 'cases.json'), encoding='utf-8'))
    failed = 0
    for case in cases:
        if args.k not in case['name']:
            continue
        try:
            problems = check_case(args.base, case)
        except Exception as e:  # 通信の失敗など
            problems = [f'例外: {e!r}']
        mark = 'OK ' if not problems else 'NG '
        print(f"{mark}{case['name']}")
        for p in problems:
            print('    ' + p)
        failed += bool(problems)
    print(f'\n{"全ケース一致" if not failed else f"{failed} ケースに差"}')
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
