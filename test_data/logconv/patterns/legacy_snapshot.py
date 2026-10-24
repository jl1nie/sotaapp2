#!/usr/bin/env python3
"""旧版(HamLogConv の convutil.py)に cases.json のケースを通して、expected/ に正解を作る。

    HAMLOGCONV_DIR=~/src/HamLogConv python3 legacy_snapshot.py

adif-io と requests が要る。公園の所在地(MY_STATE)の取得はネットワークなので外してある
(新版は、公園の所在地を、サーバーの公園データから引く。テストでは所在地を使わない)。
出力の改行は LF にそろえる。ADIF の確認(CSV読込)の結果は check.json に。
"""
import json
import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.expanduser(os.environ.get('HAMLOGCONV_DIR', '~/src/HamLogConv')))
import convutil as c  # noqa: E402

c.getPOTALoc = lambda parkid: []

BASE = {'Portable': '', 'QTH': '', 'hisQTH': '', 'hisQTHopt': '', 'myQTH': '', 'Note': '', 'Summit': '',
        'Location': '', 'WWFFOperator': '', 'WWFFActivator': '', 'WWFFRef': '', 'SOTAActivator': '',
        'POTAActivator': '', 'POTAOperator': None, 'Park': '', 'OmitPortable': ''}


def lf(text):
    return text.replace('\r\n', '\n')


def run(case):
    opts = {**BASE, **case['options']}
    path = os.path.join(HERE, case['file'])
    out = {}
    check = None
    with open(path, 'rb') as fp:
        if case['format'] == 'sota_a':
            out = c.sendSOTA_A(fp, c.decodeHamlog, opts['SOTAActivator'], opts, 'cp932', 'utf-8')
        elif case['format'] == 'sota_c':
            out = c.sendSOTA_C(fp, c.decodeHamlog, opts['callsign'] if 'callsign' in opts else case['options']['callsign'], opts, 'cp932', 'utf-8')
        else:
            out, res = c.sendADIF(fp, opts, 'cp932', 'utf-8')
            check = {'status': res['status'], 'errorlog': res['errorlog'], 'logtext': res['logtext']}
    return out, check


def main():
    cases = json.load(open(os.path.join(HERE, 'cases.json'), encoding='utf-8'))
    root = os.path.join(HERE, 'expected')
    shutil.rmtree(root, ignore_errors=True)
    for case in cases:
        files, check = run(case)
        d = os.path.join(root, case['name'])
        os.makedirs(d, exist_ok=True)
        for name, body in files.items():
            with open(os.path.join(d, name), 'w', encoding='utf-8', newline='') as f:
                f.write(lf(body))
        if check is not None:
            with open(os.path.join(d, 'check.json'), 'w', encoding='utf-8') as f:
                json.dump(check, f, ensure_ascii=False, indent=1)
                f.write('\n')
        print(f"{case['name']:34} {sorted(files)}")


if __name__ == '__main__':
    main()
