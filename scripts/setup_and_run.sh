#!/bin/bash
# AXIOM-3D setup script — run from ~/yk_rust_engine
# Downloads Gutenberg books and builds 1MB/10MB/100MB corpora

set -e
cd "$(dirname "$0")"
mkdir -p data out
cd data

echo "=== Step 1: Downloading Project Gutenberg books ==="
BOOKS="11 84 135 1661 2701 345 1342 2600 4300 100"
TOTAL=0
for id in $BOOKS; do
  fname="pg${id}.txt"
  if [ -s "$fname" ] && [ $(wc -c < "$fname" 2>/dev/null || echo 0) -gt 10000 ]; then
    echo "  $fname exists ($(wc -c < $fname) bytes), skipping"
    TOTAL=$((TOTAL + $(wc -c < $fname)))
    continue
  fi
  echo -n "  Downloading $fname... "
  curl -sS --retry 3 --retry-delay 2 -o "$fname" \
       "https://www.gutenberg.org/cache/epub/${id}/pg${id}.txt" 2>/dev/null
  if [ -s "$fname" ]; then
    SZ=$(wc -c < $fname)
    echo "OK ($SZ bytes)"
    TOTAL=$((TOTAL + SZ))
  else
    echo "FAILED"
    rm -f "$fname"
  fi
done

echo ""
echo "Total downloaded: $TOTAL bytes"
if [ $TOTAL -lt 1000000 ]; then
  echo "ERROR: Need at least 1MB but only got $TOTAL bytes. Try again in 5 minutes."
  exit 1
fi

echo ""
echo "=== Step 2: Building corpora ==="
cat pg*.txt > _all_books.txt
UNIQUE=$(wc -c < _all_books.txt)
echo "  Unique book bytes: $UNIQUE"

head -c 1048576 _all_books.txt > corpus_1mb.txt
echo "  corpus_1mb.txt:   $(wc -c < corpus_1mb.txt) bytes"

TARGET=$((10 * 1024 * 1024))
: > corpus_10mb.txt
SZ=0
while [ $SZ -lt $TARGET ]; do
  cat _all_books.txt >> corpus_10mb.txt
  SZ=$(wc -c < corpus_10mb.txt)
done
head -c $TARGET corpus_10mb.txt > _t && mv _t corpus_10mb.txt
echo "  corpus_10mb.txt:  $(wc -c < corpus_10mb.txt) bytes"

TARGET=$((100 * 1024 * 1024))
: > corpus_100mb.txt
SZ=0
while [ $SZ -lt $TARGET ]; do
  cat _all_books.txt >> corpus_100mb.txt
  SZ=$(wc -c < corpus_100mb.txt)
done
head -c $TARGET corpus_100mb.txt > _t && mv _t corpus_100mb.txt
echo "  corpus_100mb.txt: $(wc -c < corpus_100mb.txt) bytes"

echo ""
echo "=== Done. Now run: ==="
echo "  cd ~/yk_rust_engine"
echo "  cargo run --release --bin axiom3d_stress"
