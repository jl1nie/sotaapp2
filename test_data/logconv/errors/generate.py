#!/usr/bin/env python3
"""誤った入力のフィクスチャを作る(標準ライブラリだけ)。

    python3 generate.py          # このディレクトリに書く
    python3 generate.py --check  # いまのファイルが、生成の結果と一致するか確かめる(書かない)

Rust のテスト(`api/src/handler/logconv/tests/errors.rs`)が読む。
"""
import os
import struct
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))


def row(call, date="25/10/18", time="01:00U", name="", qth="", r1="", r2=""):
    return f"{call},{date},{time},59,59,7.030,CW,,,N,{name},{qth},{r1},{r2},"


def png():
    """1x1 の PNG(画像をログとして送ったとき)"""
    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(b"\x00\xff\x00\x00"))
        + chunk(b"IEND", b"")
    )


ADIF_HEAD = "ADIF Export\n<EOH>\n"
ADIF_OK = "<CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:4>0100<BAND:3>40m<MODE:2>CW"


def ref_row(call, date="25/10/18", time="01:00U"):
    return row(call, date=date, time=time, qth="JA-0002", r1="JA/TK-001 JA-0001")


def files():
    ok = row("JA1AAA", qth="JA-0002", r1="JA-0001")
    return {
        # 日付の境目(行番号: 2・3・8 が存在しない日付 = 誤り)。4 は JST でうるう日(UTC 2/29 23:00)、
        # 6 は JST の年始(UTC では前年末 23:30)
        "dates_calendar.csv": "\n".join([
            ref_row("JA1AAA", "24/02/29", "12:00U"),
            ref_row("JA1BBB", "25/02/29", "12:00U"),
            ref_row("JA1CCC", "25/02/30", "12:00U"),
            ref_row("JA1DDD", "24/03/01", "08:00J"),
            ref_row("JA1EEE", "25/12/31", "23:30U"),
            ref_row("JA1FFF", "26/01/01", "08:30J"),
            ref_row("JA1GGG", "26/01/01", "00:30U"),
            ref_row("JA1HHH", "25/04/31", "12:00U"),
        ]).encode() + b"\n",
        # 全行が誤り(存在しない時刻)
        "all_bad.csv": "\n".join(ref_row("JA1AAA", time="99:99U") if i == 0 else ref_row(f"JA1AA{i}", time="99:99U") for i in range(3)).encode() + b"\n",
        "empty.csv": b"",
        "blank_lines.csv": b"\r\n\r\n   \r\n\t\r\n",
        # HamLog iOS の見出しの行だけ
        "header_only_ios.csv": b"TimeOn,TimeOff,Freq,Callsign,RcvRST,SndRST,Name,Remarks\r\n",
        "png_like.bin": png(),
        # 2行目の引用符が閉じない(3行目以降は、その項目に飲み込まれる)
        "unclosed_quote.csv": (ok + "\n" + '"JA1BBB' + row("", qth="JA-0002", r1="JA-0001")[0:] + "\n" + ok.replace("JA1AAA", "JA1CCC") + "\n").encode(),
        # 列が15より多い行(余りは無視)
        "too_many_columns.csv": (ok + ",extra1,extra2,extra3\n" + ok.replace("JA1AAA", "JA1BBB") + "\n").encode(),
        # ADIF: 日本語のコメントを Shift_JIS で書き、長さをそのバイト数(全角2字=4)で書いたもの。UTF-8 にしたとき長さが合わない
        "adif_sjis_comment_len.adi": (
            ADIF_HEAD + ADIF_OK + "<COMMENT:4>山頂<MY_SIG_INFO:7>JA-0001<EOR>\n"
        ).encode("cp932"),
        # ADIF: QSO_DATE が全角数字(長さは 24 バイト)
        "adif_fullwidth_date.adi": (ADIF_HEAD + "<CALL:6>JA1AAA<QSO_DATE:24>２０２５１０１８<TIME_ON:4>0100<MY_SIG_INFO:7>JA-0001<EOR>\n").encode(),
        # ADIF: TIME_ON が全角数字
        "adif_fullwidth_time.adi": (ADIF_HEAD + "<CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:12>０１００<MY_SIG_INFO:7>JA-0001<EOR>\n").encode(),
        # ADIF: ヘッダーに ŉ(大文字にすると1バイト増える字)がある。大文字にした文字列の位置を元の文字列に使うと、記録が欠ける
        "adif_header_special_char.adi": (
            "ADIF " + "ŉ" * 5 + "\n<EOH>\n" + ADIF_OK + "<MY_SIG_INFO:7>JA-0001<EOR>\n"
        ).encode(),
        # ADIF: 存在しない日付(13月45日)と時刻(99:99)
        "adif_nonexistent_datetime.adi": (ADIF_HEAD + "<CALL:6>JA1AAA<QSO_DATE:8>20251345<TIME_ON:4>9999<BAND:3>40m<MODE:2>CW<MY_SIG_INFO:7>JA-0001<EOR>\n").encode(),
        # ADIF: <EOR> がない(最後の記録が閉じていない)
        "adif_no_eor.adi": (ADIF_HEAD + ADIF_OK + "<MY_SIG_INFO:7>JA-0001").encode(),
    }


def main():
    check = "--check" in sys.argv
    bad = 0
    for name, data in sorted(files().items()):
        path = os.path.join(HERE, name)
        if check:
            same = os.path.exists(path) and open(path, "rb").read() == data
            print(("OK  " if same else "NG  ") + name)
            bad += not same
        else:
            with open(path, "wb") as f:
                f.write(data)
            print(f"wrote {name} ({len(data)} bytes)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
