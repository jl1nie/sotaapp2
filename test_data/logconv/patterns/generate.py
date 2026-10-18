#!/usr/bin/env python3
"""logconv のテストデータ(パターンを網羅したフィクスチャ)を作る。

    python3 generate.py

HAMLOG CSV の列: コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,(15列目)
自局の参照は備考1、相手局の参照は QTH(と備考2に同じ内容)に置く。
SOTA アクティベーターの画面は相手を備考2で、POTA の画面は QTH で指定するため、両方の列に入れてある。
"""
import csv
import itertools
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def write_csv(name, rows, encoding='utf-8'):
    with open(os.path.join(HERE, name), 'w', newline='', encoding=encoding) as f:
        w = csv.writer(f, lineterminator='\n')
        for r in rows:
            w.writerow(r)


_calls = itertools.count()


def call():
    """JA1AAA, JA1AAB, ... の順に、重ならないコールサインを返す"""
    n = next(_calls)
    return 'JA1' + chr(65 + n // 676 % 26) + chr(65 + n // 26 % 26) + chr(65 + n % 26)


def row(callsign, date, time, freq, mode, qth='', mine='', rs='59', rr='59', name='', rmks2=None, gl=''):
    # 相手の参照は、QTH の列と備考2の列の両方に(画面の指定に合わせて、どちらでも読めるように)
    return [callsign, date, time, rs, rr, freq, mode, '', gl, 'N', name, qth, mine,
            qth if rmks2 is None else rmks2, '']


MINE = 'JA/TK-001 JA-0001'  # 自局: 山と公園

# ---- 1. バンドとモード ----
BANDS = [0.1370, 0.475, 1.9, 3.5, 3.575, 3.8, 7.025, 7.1, 10.12, 14.074, 14.2, 18.1, 21.074,
         24.915, 28.074, 29.6, 50.313, 51.0, 144.1, 145.5, 430.5, 435.0, 1296.1, 2400.5, 5700.0,
         10100.0, 10475.0]
MODES = ['CW', 'SSB', 'FM', 'AM', 'RTTY', 'FT8', 'FT4', 'PSK31', 'JT65', 'JT9', 'DV', 'C4FM',
         'D-STAR', 'DMR', 'JS8', 'FST4', 'Q65', 'FUSION', 'xyz']
rows = []
minute = 0
for i, f in enumerate(BANDS):  # 全バンド(モードは順に)
    rows.append(row(call(), '25/10/18', f'01:{minute % 60:02d}U', f'{f}', MODES[i % len(MODES)], mine=MINE))
    minute += 1
for m in MODES:  # 全モード(14.2 と 7.1 で、SSB の LSB/USB の判定も)
    rows.append(row(call(), '25/10/18', f'02:{minute % 60:02d}U', '14.2', m, mine=MINE))
    minute += 1
    rows.append(row(call(), '25/10/18', f'03:{minute % 60:02d}U', '7.1', m, mine=MINE))
    minute += 1
rows.append(row(call(), '25/10/18', '04:00U', '14.2', 'SSB', mine=MINE, rs='5', rr='9'))  # RST が1桁
rows.append(row(call(), '25/10/18', '04:01U', '14.074', 'FT8', mine=MINE, rs='-10', rr='+03'))  # SNR
rows.append(row(call(), '25/10/18', '04:02U', '14.2', 'SSB', mine=MINE, rs='', rr=''))  # RST なし
rows.append(row(call(), '25/10/18', '04:03U', '7.030/7.035', 'CW', mine=MINE))  # 周波数が「a/b」
write_csv('bands_modes.csv', rows)

# ---- 2. 参照番号(自局・相手の組み合わせ、書式) ----
REFS = [
    # (自局の参照, 相手の参照, 説明)
    ('JA/TK-001', '', '山だけ・相手なし'),
    (MINE + ' JAFF-0001', '', '山・公園・WWFF・相手なし'),
    (MINE, 'JA/KN-001', 'S2S'),
    (MINE, 'JA-0123', 'P2P'),
    (MINE + ' JAFF-0001', 'JAFF-0456', 'W2W'),
    (MINE + ' JAFF-0001', 'JA/KN-001 JA-0123 JAFF-0456', 'S2S + P2P + W2W'),
    (MINE, 'JA-0123 JA-0124', '相手 2-fer'),
    (MINE, 'JA-0123,JA-0124,JA-0125', '相手 3-fer(カンマ区切り)'),
    (MINE, 'ja/kn-001 ja-0123', '小文字'),
    ('JA/TK-001 JA-0001 JA-0002', 'JA-0123 JA-0124', '自局 2-fer × 相手 2-fer'),
    ('JA/TK-001 JA-0001 JA-0002 JA-0003', 'JA-0123 JA-0124 JA-0125', '自局 3-fer × 相手 3-fer'),
    ('JA/TK-001 JA-0001 JA-0002', '', '自局 2-fer・相手なし'),
    ('JA-0003 JAFF-0003', '', '公園・WWFF だけ(山なし)'),
    ('JAFF-0004', 'JAFF-0457', 'WWFF だけ'),
    ('JA/TK-002 JA-0001', 'JA/KN-002', '山を移動(JA/TK-002)'),
    (MINE, 'JA/KN-001 35.6N, 139.2E', '相手の場所の座標つき'),
    (MINE, 'JA/KN-001  JA-0123', '空白が2つ'),
    (MINE, 'JA/KN-001,JA-0123', 'カンマだけで区切る'),
    ('', '', '自局の参照なし'),
    ('', 'JA/KN-001', '自局の参照なし・相手あり'),
    (MINE, 'JL/MH-005', '山の地域が JA 以外(JL)'),
    (MINE, 'JA-0123 (TNX)', '参照のあとにメモ'),
]
rows = []
for i, (mine, his, _) in enumerate(REFS):
    rows.append(row(call(), '25/10/18', f'01:{i:02d}U', ['7.030', '14.285', '21.300', '28.500'][i % 4],
                    ['CW', 'SSB', 'SSB', 'FM'][i % 4], qth=his, mine=mine))
write_csv('references.csv', rows)

# 自局の参照が備考2、相手の参照が備考1(画面の「Remarks2で指定する」)。QTH は相手と別の内容
rows = []
for i, (mine, his, _) in enumerate(REFS):
    rows.append(row(call(), '25/10/18', f'01:{i:02d}U', ['7.030', '14.285', '21.300', '28.500'][i % 4],
                    ['CW', 'SSB', 'SSB', 'FM'][i % 4], qth='Tokyo', mine=his, rmks2=mine))
write_csv('references_my_rmks2.csv', rows)

# ---- 3. 日付・時刻・タイムゾーン ----
# 旧版は、UTC の日付が変わるたびにファイルを書き出し、日付が戻ると上書きして交信が消える(新版は直した)。
# そのため、旧版と比べるこのファイルは、UTC の時刻順に並べてある。順不同のログは unordered.csv
import datetime


def utc_of(date, time):
    y, m, d = (int(x) for x in date.split('/'))
    y = y if y > 99 else (1900 + y if y >= 65 else 2000 + y)
    hh, mm, flag = int(time[0:2]), int(time[3:5]), time[5:] or 'J'
    offset = 0 if flag in ('U', 'Z') else 9
    return datetime.datetime(y, m, d, hh, mm) - datetime.timedelta(hours=offset)


TZ = [
    ('25/10/18', '23:50U'), ('25/10/19', '00:05U'),            # UTC で日付をまたぐ
    ('25/10/19', '08:50J'), ('25/10/19', '09:10J'),            # JST。UTC では 10/18 23:50 と 10/19 00:10
    ('25/10/19', '03:00Z'),                                    # Z(UTC)
    ('25/10/20', '10:00U'),                                    # 3日目
    ('2025/10/20', '11:00U'),                                  # 4桁の年
    ('99/12/31', '23:59U'),                                    # 2桁の年(65 以上は 19xx)
    ('25/12/31', '23:50U'), ('26/01/01', '00:10U'),            # 年またぎ
    ('26/01/01', '08:50J'),                                    # JST で年をまたぐ(UTC は 12/31 23:50)
    ('24/02/29', '12:00U'),                                    # うるう日
    ('25/10/19', '00:00J'),                                    # JST の 0 時(UTC は前日 15:00)
    ('25/10/18', '12:00U'),
    ('25/10/21', '23:59U'),
]
TZ.sort(key=lambda dt: utc_of(*dt))
rows = [row(call(), d, t, '7.030', 'CW', qth='JA/KN-001' if i % 3 == 0 else '', mine=MINE)
        for i, (d, t) in enumerate(TZ)]
write_csv('multiday_tz.csv', rows)

# 順不同(日付が行き来する)のログ。新版は、どの交信も消さずに、日付ごとのファイルに集める
UNORDERED = [('25/10/18', '10:00U'), ('25/10/19', '10:00U'), ('25/10/18', '10:05U'),
             ('25/10/20', '10:00U'), ('25/10/19', '10:05U'), ('25/10/18', '10:10U')]
rows = [row(call(), d, t, '7.030', 'CW', qth='JA/KN-001' if i % 2 == 0 else '', mine=MINE)
        for i, (d, t) in enumerate(UNORDERED)]
write_csv('unordered.csv', rows)

# ---- 4. 誤りのある行(誤りの行と正しい行が混ざる) ----
GOOD = lambda t: row(call(), '25/10/18', t, '7.030', 'CW', qth='JA/KN-001', mine=MINE)
rows = [
    GOOD('01:00U'),
    row(call(), '25-10-18', '01:05U', '7.030', 'CW', mine=MINE),        # 日付の書式
    row(call(), '25/10/18', 'ab:cd', '7.030', 'CW', mine=MINE),         # 時刻の書式
    row(call(), '25/10/18', '25:61U', '7.030', 'CW', mine=MINE),        # 存在しない時刻
    row(call(), '25/13/45', '01:10U', '7.030', 'CW', mine=MINE),        # 存在しない日付
    row(call(), '25/10/18', '01:15U', '9.999', 'CW', mine=MINE),        # バンドにない周波数
    row(call(), '25/10/18', '01:20U', 'abc', 'CW', mine=MINE),          # 周波数が数字でない
    ['JA1SHT', '25/10/18', '01:25U', '59', '59'],                       # 列が足りない行
    [],                                                                 # 空行
    GOOD('01:30U'),
]
write_csv('errors.csv', rows)

# ---- 5. Shift-JIS(日本語版 HAMLOG の CSV) ----
SJIS = [
    row(call(), '25/10/18', '01:00U', '7.030', 'CW', qth='東京都八王子市', mine='JA/TK-001 高尾山', name='山田太郎', rmks2='ありがとう'),
    row(call(), '25/10/18', '01:05U', '7.030', 'CW', qth='JA/KN-001 神奈川', mine='JA/TK-001 高尾山 JA-0001', name='鈴木 花子'),
    row(call(), '25/10/18', '01:10U', '14.285', 'SSB', qth='JA-0123 表', mine=MINE, name='能'),   # 2バイト目が 0x5C の字(表・能)
    row(call(), '25/10/18', '01:15U', '14.285', 'SSB', qth='ソフト JA/KN-002', mine=MINE, name='ソ'),  # ソ(0x835C)
]
write_csv('sjis.csv', SJIS, encoding='cp932')

# ---- 6. ADIF を入力にする ----
def adif(**f):
    return ''.join(f'<{k.upper()}:{len(v)}>{v}' for k, v in f.items()) + '<EOR>\n'


records = [
    adif(call='JA1ADA', qso_date='20251018', time_on='0100', band='40m', mode='CW', rst_sent='599', rst_rcvd='599',
         my_sig_info='JA-0001', sig_info='JA-0123'),
    adif(call='JA1ADB', qso_date='20251018', time_on='0105', band='20m', mode='SSB', rst_sent='59', rst_rcvd='59',
         my_sota_ref='JA/TK-001', sota_ref='JA/KN-001'),
    adif(call='JA1ADC', qso_date='20251018', time_on='0110', freq='14.074', mode='FT8', submode='', rst_sent='-10',
         rst_rcvd='-12', my_sig_info='JA-0001'),
    adif(call='JA1ADD', qso_date='20251018', time_on='011500', band='15m', mode='SSB', my_sota_ref='JA/TK-001'),
    adif(call='JA1ADE', qso_date='20251018', time_on='0120', band='40m', mode='MFSK', submode='FT4', my_sig_info='JA-0001', sig_info='JA-0124'),
    adif(call='JA1ADF', qso_date='20251019', time_on='0000', band='10m', mode='FM', my_sig_info='JA-0002', comment='TNX'),
]
with open(os.path.join(HERE, 'input.adi'), 'w', encoding='utf-8', newline='') as f:
    f.write('ADIF export from a logger\n<ADIF_VER:5>3.1.4<PROGRAMID:4>TEST\n<EOH>\n')
    f.writelines(records)

# ---- 7. HamLog iOS 形式 ----
IOS_HEADER = ['TimeOn', 'x', 'Freq', 'Call', 'RSTr', 'RSTs', 'Grid', 'Name', 'Qth', 'x9', 'x10', 'Mode', 'x12',
              'Rmks2', 'QSL', 'QSLs', 'QSLr', 'x17', 'x18']


def ios(dt, freq, callsign, rr, rs, qth, mode, rmks2):
    c = [''] * 19
    c[0], c[2], c[3], c[4], c[5], c[8], c[11], c[13] = dt, freq, callsign, rr, rs, qth, mode, rmks2
    c[14], c[15], c[16] = 'N', '0', '0'
    return c


rows = [IOS_HEADER,
        ios('2025-10-18 10:00:00 +0900', '7.030', call(), '59', '59', MINE, 'CW', ''),
        ios('2025-10-18 10:05:00 +0900', '7.030', call(), '59', '59', MINE, 'CW', 'JA/KN-001'),
        ios('2025-10-18 10:10:00 +0900', '14.285', call(), '59', '59', MINE, 'SSB', 'JA-0123 JA-0124'),
        ios('2025-10-18 10:15:00 +0900', '10.136', call(), '-10', '-12', MINE, 'FT8', ''),
        ios('2025-10-19 08:30:00 +0900', '21.300', call(), '59', '59', MINE, 'SSB', 'JA-0125')]  # UTC で前日
write_csv('ios.csv', rows)
print('生成:', sorted(f for f in os.listdir(HERE) if f.endswith(('.csv', '.adi'))))
