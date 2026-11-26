#!/usr/bin/env python3
"""Build crates/poni-core/data/eve-kspace.bin.zst from Fuzzwork SDE CSV dumps.

Binary format (little-endian):
  u16 system_count
  repeated system_count times:
    u8 name_len
    [u8; name_len] UTF-8 solarSystemName
  u32 edge_count
  repeated edge_count times:
    u16 from_idx
    u16 to_idx
"""

import bz2
import csv
import io
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

SYSTEMS_URL = "https://www.fuzzwork.co.uk/dump/latest/csv/mapSolarSystems.csv"
JUMPS_URL = "https://www.fuzzwork.co.uk/dump/latest/csv/mapSolarSystemJumps.csv"

ROOT = Path(__file__).resolve().parent.parent
OUT_ZST = ROOT / "crates" / "poni-core" / "data" / "eve-kspace.bin.zst"


def fetch_csv_rows(url: str):
    print(f"Downloading {url} ...", flush=True)
    req = urllib.request.Request(url, headers={"User-Agent": "PulauH-EveMapBuilder/1.0"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = resp.read()
    if url.endswith(".bz2"):
        data = bz2.decompress(data)
    text = data.decode("utf-8-sig", errors="replace")
    return list(csv.DictReader(io.StringIO(text)))


def main():
    systems_rows = fetch_csv_rows(SYSTEMS_URL)
    jumps_rows = fetch_csv_rows(JUMPS_URL)

    # Keep all k-space + named EVE systems (regionID < 11000000 is k-space;
    # keep any system that appears in k-space or has stargates, plus J-space?
    # Note: J-space has regionID >= 11000000 and no stargates, while Pochven
    # regionID is 10000070 < 11000000 and HAS stargates).
    id_to_name = {}
    for row in systems_rows:
        sid = int(row["solarSystemID"])
        rid = int(row["regionID"])
        name = row["solarSystemName"].strip()
        if rid < 11000000 and name:
            id_to_name[sid] = name

    # Also ensure any endpoint of a stargate jump is included.
    raw_edges = set()
    for row in jumps_rows:
        a = int(row["fromSolarSystemID"])
        b = int(row["toSolarSystemID"])
        if a == b:
            continue
        if a in id_to_name and b in id_to_name:
            edge = (min(a, b), max(a, b))
            raw_edges.add(edge)

    sorted_ids = sorted(id_to_name.keys(), key=lambda sid: id_to_name[sid].lower())
    idx_by_id = {sid: i for i, sid in enumerate(sorted_ids)}

    buf = bytearray()
    buf += struct.pack("<H", len(sorted_ids))
    for sid in sorted_ids:
        name_bytes = id_to_name[sid].encode("utf-8")
        if len(name_bytes) > 255:
            raise ValueError(f"System name too long: {id_to_name[sid]}")
        buf.append(len(name_bytes))
        buf += name_bytes

    edges = sorted((idx_by_id[a], idx_by_id[b]) for (a, b) in raw_edges)
    buf += struct.pack("<I", len(edges))
    for u, v in edges:
        buf += struct.pack("<HH", u, v)

    zstd_bin = shutil.which("zstd")
    if not zstd_bin:
        print("ERROR: zstd executable not found in PATH", file=sys.stderr)
        sys.exit(1)

    OUT_ZST.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(delete=False, suffix=".bin") as tmp:
        tmp.write(buf)
        tmp_path = tmp.name

    try:
        subprocess.run([zstd_bin, "-19", "-f", tmp_path, "-o", str(OUT_ZST)], check=True)
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)

    print(
        f"Built {OUT_ZST}: {len(sorted_ids)} systems, {len(edges)} edges, "
        f"raw={len(buf)} bytes, zst={OUT_ZST.stat().st_size} bytes"
    )


if __name__ == "__main__":
    main()
