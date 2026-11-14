#!/usr/bin/env python3
"""cargo llvm-covのHTMLレポートを別PCのブラウザーから確認する。"""

import argparse
import sys
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent.parent


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bind", default="0.0.0.0", help="待受アドレス（既定: 0.0.0.0）"
    )
    parser.add_argument(
        "--port", type=int, default=8000, help="ポート番号（既定: 8000、0なら自動選択）"
    )
    parser.add_argument(
        "--directory",
        type=Path,
        default=Path("target/llvm-cov/html"),
        help="配信ディレクトリ。相対パスはリポジトリ基準（既定: target/llvm-cov/html）",
    )
    args = parser.parse_args()
    if not 0 <= args.port <= 65535:
        parser.error("ポート番号には0〜65535を指定してください")

    directory = args.directory.expanduser()
    if not directory.is_absolute():
        directory = PROJECT_ROOT / directory
    directory = directory.resolve()
    if not (directory / "index.html").is_file():
        parser.error(
            f"{directory / 'index.html'} がありません。"
            "先に cargo llvm-cov --locked --html でレポートを生成するか、"
            "--directoryでHTMLのあるディレクトリを指定してください"
        )

    handler = partial(SimpleHTTPRequestHandler, directory=str(directory))
    try:
        with ThreadingHTTPServer((args.bind, args.port), handler) as server:
            port = server.server_port
            local_host = "127.0.0.1" if args.bind == "0.0.0.0" else args.bind
            print(f"配信ディレクトリ: {directory}")
            print(f"待受: {args.bind}:{port}")
            print(f"確認用URL: http://{local_host}:{port}/")
            if args.bind == "0.0.0.0":
                print(f"別PCから: http://<このPCのLAN-IP>:{port}/")
            print("終了するにはCtrl+Cを押してください。", flush=True)
            server.serve_forever()
    except KeyboardInterrupt:
        print("\nサーバーを停止しました。")
    except OSError as error:
        print(f"サーバーエラー: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
