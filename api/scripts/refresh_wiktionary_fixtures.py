#!/usr/bin/env python3
"""Refresh wikitext fixture files for the wiktionary parser tests.

Each fixture is downloaded from the corresponding Wiktionary edition's Core REST API.
Attribution: All Wiktionary content is CC-BY-SA 3.0 / GFDL.

Usage:
    python3 scripts/refresh_wiktionary_fixtures.py
    python3 scripts/refresh_wiktionary_fixtures.py --lang pt
    python3 scripts/refresh_wiktionary_fixtures.py --dry-run
"""

import argparse
import json
import os
import sys
import time
import urllib.request

API = "https://{subdomain}.wiktionary.org/w/rest.php/v1/page/{word}"
USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36"
DELAY = 1.5 

FIXTURES = {
    "en/heart.txt":     ("en", "heart"),
    "en/run.txt":       ("en", "run"),
    "en/coração.txt":   ("en", "coração"),
    "en/banco.txt":     ("en", "banco"),
    "en/cuore.txt":     ("en", "cuore"),
    "en/Wasser.txt":    ("en", "Wasser"),
    "en/maison.txt":    ("en", "maison"),
    "en/empurro.txt":   ("en", "empurro"),
    "pt/coração.txt":   ("pt", "coração"),
    "pt/banco.txt":     ("pt", "banco"),
    "pt/cantar.txt":    ("pt", "cantar"),
    "pt/empurro.txt":   ("pt", "empurro"),
    "it/buongiorno.txt": ("it", "buongiorno"),
    "it/banca.txt":     ("it", "banca"),
    "it/cuore.txt":     ("it", "cuore"),
    "it/parlare.txt":   ("it", "parlare"),
    "de/Wasser.txt":    ("de", "Wasser"),
    "de/Bank.txt":      ("de", "Bank"),
    "de/Haus.txt":      ("de", "Haus"),
    "de/gehen.txt":     ("de", "gehen"),
    "es/banco.txt":     ("es", "banco"),
    "es/hola.txt":      ("es", "hola"),
    "es/hablar.txt":    ("es", "hablar"),
    "es/casa.txt":      ("es", "casa"),
    "fr/banque.txt":    ("fr", "banque"),
    "fr/bonjour.txt":   ("fr", "bonjour"),
    "fr/maison.txt":    ("fr", "maison"),
    "fr/manger.txt":    ("fr", "manger"),
}


def get_test_data_dir():
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(repo_root, "test_data")


def fetch_wikitext(subdomain: str, word: str) -> str:
    url = API.format(subdomain=subdomain, word=urllib.parse.quote(word, safe=""))
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    source = data.get("source", "")
    if not source:
        print(f"    WARNING: empty source for {subdomain}:{word}", file=sys.stderr)
    return source


def refresh(lang_filter: str | None, dry_run: bool):
    base = get_test_data_dir()
    os.makedirs(base, exist_ok=True)
    os.makedirs(os.path.join(base, "en"), exist_ok=True)

    for output_path, (subdomain, word) in sorted(FIXTURES.items()):
        lang = output_path.split("/")[0]
        if lang_filter and lang != lang_filter:
            continue

        out_file = os.path.join(base, output_path)
        os.makedirs(os.path.dirname(out_file), exist_ok=True)

        if dry_run:
            print(f"[DRY RUN] Would fetch {subdomain}:{word} -> {output_path}")
            continue

        print(f"Fetching {subdomain}:{word} ...", end=" ", flush=True)
        try:
            wikitext = fetch_wikitext(subdomain, word)
            with open(out_file, "w") as f:
                f.write(wikitext)
            print(f"{len(wikitext)}b written to {output_path}")
        except Exception as e:
            print(f"FAILED: {e}", file=sys.stderr)

        time.sleep(DELAY)

    print("\nDone. Run `cargo test -- wiktionary_parser` to verify.")


if __name__ == "__main__":
    import urllib.parse

    parser = argparse.ArgumentParser(description="Refresh Wiktionary test fixtures")
    parser.add_argument("--lang", help="Only refresh fixtures for this language code (en, pt, it, de, es, fr)")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be fetched without downloading")
    args = parser.parse_args()

    refresh(args.lang, args.dry_run)
