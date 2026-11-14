"""Regenerate the word-frequency lists the English auto-restore reads.

    python3 -m venv /tmp/wf && /tmp/wf/bin/pip install wordfreq==3.1.1
    /tmp/wf/bin/python crates/vnkey-engine/data/gen_wordlists.py

Writes (paths relative to the repo root):
  crates/vnkey-engine/data/en_words.txt      "word zipf*10", English
  crates/vnkey-engine/data/vi_syllables.txt  "telex zipf*10", every Vietnamese
      syllable with a diacritic (zipf 0 when wordfreq has no count)
  crates/vnkey-engine/tests/data/vi_typing.txt  the same syllables (plus plain
      ones) as typing test cases, two key orders
  crates/vnkey-engine/tests/data/en_heldout.txt English NOT in en_words.txt,
      so the tests can measure what happens to words the dictionary misses

Vietnamese syllables come from the hunspell-vi lexicon (so a syllable that is
rare on the web but real, like "dóc" or "tét", is still known) plus the common
ones in wordfreq. They are keyed by their canonical Telex spelling (modifiers
doubled, tone key last) so the engine can look a composed word up without
caring about old/modern tone placement. See README.md for the data licence.
"""

import pathlib
import re
import unicodedata
import urllib.request

import wordfreq

ROOT = pathlib.Path(__file__).resolve().parents[3]
DATA = ROOT / "crates/vnkey-engine/data"
TEST_DATA = ROOT / "crates/vnkey-engine/tests/data"

HUNSPELL_VI = (
    "https://raw.githubusercontent.com/1ec5/hunspell-vi/"
    "39cc647f0cf3bdfc579980a2c3aac3f6f4203fe6/dictionaries/vi-DauMoi.dic"
)
VI_MIN_ZIPF = 3.0
EN_MIN_ZIPF = 3.0
EN_HELDOUT_MIN_ZIPF = 2.5
# Tokens in the source lists that are not Vietnamese syllables.
VI_NOT_SYLLABLES = {"piô", "gỵa"}
# No tone on a c/ch/p/t final is not a finished syllable ("mêt" is "mệt" with
# the tone forgotten); the engine judges those by their toned forms instead.
TONELESS_STOP = re.compile(r"^[^\u0300\u0301\u0303\u0309\u0323]*(c|ch|p|t)$")
# A word Telex can never transform never needs restoring, so leave it out:
# it has no tone/horn/z key and no repeated a/e/o/d (circumflex, stroke).
TELEX_TRIGGER = re.compile(r"[sfrxjwz]|([aeod]).*\1")

TONES = {"\u0301": "s", "\u0300": "f", "\u0309": "r", "\u0303": "x", "\u0323": "j"}
DOUBLED = "\u0302"  # circumflex: aa ee oo
HORNED = ("\u0306", "\u031b")  # breve, horn: aw ow uw

V = "aàáảãạăằắẳẵặâầấẩẫậeèéẻẽẹêềếểễệiìíỉĩịoòóỏõọôồốổỗộơờớởỡợuùúủũụưừứửữựyỳýỷỹỵ"
SYLLABLE = re.compile(
    rf"^(ngh|ng|nh|ch|gh|gi|kh|ph|qu|th|tr|[bcdđghklmnpqrstvx])?[{V}]+(ch|ng|nh|[cmnpt])?$"
)


def telex(syllable: str, tone_last: bool = True) -> str:
    keys, tone = [], ""
    for ch in syllable:
        if ch == "đ":
            keys.append("dd")
            continue
        base, *marks = unicodedata.normalize("NFD", ch)
        # NFD may put the tone before the circumflex (ộ = o + dot + hat), so
        # build the letter first and add its tone key after.
        part = base + (base if DOUBLED in marks else "") + ("w" if set(marks) & set(HORNED) else "")
        for m in marks:
            if m in TONES:
                tone = TONES[m]
                if not tone_last:
                    part += tone
        keys.append(part)
    return "".join(keys) + (tone if tone_last else "")


def main() -> None:
    en, heldout = {}, []
    for w in wordfreq.top_n_list("en", 60000):
        z = wordfreq.zipf_frequency(w, "en")
        if not (re.fullmatch(r"[a-z]{2,}", w) and TELEX_TRIGGER.search(w)):
            continue
        if z >= EN_MIN_ZIPF:
            en[w] = round(z * 10)
        elif z >= EN_HELDOUT_MIN_ZIPF:
            heldout.append(w)
    (DATA / "en_words.txt").write_text("".join(f"{w} {z}\n" for w, z in sorted(en.items())))
    # Every 6th word keeps the test data small while still covering the band.
    heldout = sorted(heldout)[::6]

    with urllib.request.urlopen(HUNSPELL_VI) as f:
        lexicon = {
            unicodedata.normalize("NFC", line.split("/")[0].strip()).lower()
            for line in f.read().decode("utf-8").splitlines()[1:]
        }
    for w in wordfreq.top_n_list("vi", 100000):
        w = unicodedata.normalize("NFC", w)
        if wordfreq.zipf_frequency(w, "vi") >= VI_MIN_ZIPF:
            lexicon.add(w)

    vi, typing = {}, set()
    for w in sorted(lexicon):
        if not SYLLABLE.match(w) or w in VI_NOT_SYLLABLES:
            continue
        # Plain-ASCII tokens include English that leaks into Vietnamese web
        # text ("this", "eye"). Typed in Telex those compose into something
        # else, so they are neither a typing case nor a syllable key; a plain
        # token with no Telex trigger ("ba", "cho") types as itself and stays.
        if w.isascii():
            if not TELEX_TRIGGER.search(w):
                typing.add((w, w))
            continue
        if TONELESS_STOP.match(unicodedata.normalize("NFD", w)):
            continue
        typing.add((telex(w), telex(w, tone_last=False)))
        vi[telex(w)] = round(wordfreq.zipf_frequency(w, "vi") * 10)
    (DATA / "vi_syllables.txt").write_text("".join(f"{k} {z}\n" for k, z in sorted(vi.items())))

    TEST_DATA.mkdir(parents=True, exist_ok=True)
    (TEST_DATA / "vi_typing.txt").write_text("".join(f"{a}\t{b}\n" for a, b in sorted(typing)))
    (TEST_DATA / "en_heldout.txt").write_text("".join(f"{w}\n" for w in heldout))
    print(f"en={len(en)} vi={len(vi)} vi_typing={len(typing)} en_heldout={len(heldout)}")


if __name__ == "__main__":
    main()
