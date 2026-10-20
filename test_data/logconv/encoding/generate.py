#!/usr/bin/env python3
"""文字コード・改行・末尾のバイト列のフィクスチャを作る(標準ライブラリだけ)。

    python3 generate.py          # このディレクトリに書く
    python3 generate.py --check  # いまのファイルが、生成の結果と一致するか確かめる(書かない)

HAMLOG CSV の列は `コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,`。
Rust のテスト(`api/src/handler/logconv/tests/encoding.rs`)が読む。
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def row(call, name="", qth="", r1="", r2="", date="25/10/18", time="01:00U"):
    return f"{call},{date},{time},59,59,7.030,CW,,,N,{name},{qth},{r1},{r2},"


# 日本語の名前・QTH・備考の行。自局の参照は備考1、相手の参照は QTH。
# 表・能・ソ・十・予・﨑 は、2バイト目が 0x5C(バックスラッシュ)の字。
NAMES = [
    row("JA1AAA", "表能ソ十予", "JA/KN-006 JA-0002 表", "JA/TK-001 JA-0001 能", "ソ予"),
    row("JA1BBB", "山田太郎", "JA-0003", "JA/TK-001 JA-0001 十分", "予定"),
    row("JA1CCC", "ｱｲｳｴｵ ﾊﾝｶｸ", "ｶﾅ JA-0004", "JA/TK-001 JA-0001 ﾃｽﾄ", ""),
    row("JA1DDD", "①㈱髙﨑№", "JA-0005 髙", "JA/TK-001 JA-0001 﨑", "№～＼"),
]

# 日付の列に日本語を入れた行(誤りの知らせに、その文字がそのまま出る)。2行目と3行目が誤り。
ERROR_TEXT = [
    row("JA1AAA", "", "JA-0002", "JA-0001"),
    row("JA1BBB", "", "JA-0002", "JA-0001", date="令和7年10月18日①㈱髙﨑№"),
    row("JA1CCC", "", "JA-0002", "JA-0001", date="表能ソ十予/ｱｲｳ"),
    row("JA1DDD", "", "JA-0002", "JA-0001"),
]
ERROR_TEXT_MESSAGES = ["令和7年10月18日①㈱髙﨑№", "表能ソ十予/ｱｲｳ"]


def crlf(rows):
    return "\r\n".join(rows) + "\r\n"


def utf16(text, order, bom):
    data = text.encode("utf-16-" + order)  # BOM なし
    if bom:
        data = (b"\xff\xfe" if order == "le" else b"\xfe\xff") + data
    return data


def files():
    names = crlf(NAMES)
    errs = crlf(ERROR_TEXT)
    plain = crlf([row("JA1AAA", "", "JA-0002", "JA-0001"), row("JA1BBB", "", "JA-0003", "JA-0001")])
    out = {
        # Shift_JIS(cp932)。日本語版 HAMLOG の CSV
        "sjis_names.csv": names.encode("cp932"),
        "sjis_error_text.csv": errs.encode("cp932"),
        # 同じ内容の、ほかの文字コード
        "utf8_names.csv": names.encode("utf-8"),
        "utf8_bom_names.csv": b"\xef\xbb\xbf" + names.encode("utf-8"),
        "utf8_error_text.csv": errs.encode("utf-8"),
        "utf8_bom_error_text.csv": b"\xef\xbb\xbf" + errs.encode("utf-8"),
        "utf16le_bom_names.csv": utf16(names, "le", True),
        "utf16be_bom_names.csv": utf16(names, "be", True),
        "utf16le_nobom_names.csv": utf16(names, "le", False),
        # Shift_JIS の並びが、偶然 UTF-8 としても正しい例。日付の列が `縺ｱ`(E3 81 B1)。UTF-8 なら `ぱ`
        "sjis_utf8_lookalike.csv": (
            crlf([row("JA1AAA", "", "JA-0002", "JA-0001", date="縺ｱ"), row("JA1BBB", "", "JA-0003", "JA-0001")])
        ).encode("cp932"),
        # 壊れたバイトを含む Shift_JIS。2行目の名前に 0xFF、3行目の名前の終わりに 83(2バイト文字の1バイト目だけ)、
        # 4行目の行末(最後の列の後ろ)に 81、5行目の日付が `25<FF>10<FF>18`(この行だけが誤り)
        "sjis_invalid_bytes.csv": (
            row("JA1AAA", "", "JA-0002", "JA-0001").encode("cp932") + b"\r\n"
            + row("JA1BBB", "\x01", "JA-0003", "JA-0001").encode("cp932").replace(b"\x01", b"ab\xff\xfe") + b"\r\n"
            + row("JA1CCC", "\x01", "JA-0004", "JA-0001").encode("cp932").replace(b"\x01", b"\x83") + b"\r\n"
            + row("JA1DDD", "", "JA-0005", "JA-0001").encode("cp932") + b"\x81\r\n"
            + row("JA1EEE", "", "JA-0006", "JA-0001", date="25\x0110\x0118").encode("cp932").replace(b"\x01", b"\xff") + b"\r\n"
        ),
        # UTF-8 の日本語に、壊れたバイト(0xFF)が1つ。ファイル全体が Shift_JIS として読まれる
        "utf8_invalid_byte.csv": (
            row("JA1AAA", "山田太郎", "JA-0002", "JA-0001").encode("utf-8") + b"\n"
            + row("JA1BBB", "\x01", "JA-0003", "JA-0001").encode("utf-8").replace(b"\x01", b"\xff") + b"\n"
            + row("JA1CCC", "鈴木", "JA-0004", "JA-0001").encode("utf-8") + b"\n"
        ),
        # 改行
        "newline_crlf.csv": plain.encode("ascii"),
        "newline_lf.csv": plain.replace("\r\n", "\n").encode("ascii"),
        "newline_cr.csv": plain.replace("\r\n", "\r").encode("ascii"),
        "newline_none_at_end.csv": plain.rstrip("\r\n").encode("ascii"),
        "newline_mixed.csv": (row("JA1AAA", "", "JA-0002", "JA-0001") + "\r\n" + row("JA1BBB", "", "JA-0003", "JA-0001") + "\n"
                              + row("JA1CCC", "", "JA-0004", "JA-0001") + "\r\n").encode("ascii"),
        # 末尾の 0x1A(古い DOS の EOF)
        "sub_eof_after_newline.csv": plain.encode("ascii") + b"\x1a",
        "sub_eof_no_newline.csv": plain.rstrip("\r\n").encode("ascii") + b"\x1a",
        # ASCII だけ
        "ascii_only.csv": plain.encode("ascii"),
    }
    return out


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
