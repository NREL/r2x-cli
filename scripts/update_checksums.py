import glob
import hashlib
import json
import os
import sys

manifest_path = sys.argv[1]

hashes = {}

# Update .sha256 files
for pattern in ["target/distrib/r2x-*.tar.xz", "target/distrib/r2x-*.zip"]:
    for file in glob.glob(pattern):
        print(f"Updating checksum for {file}")
        with open(file, "rb") as f:
            h = hashlib.sha256(f.read()).hexdigest()
        hashes[os.path.basename(file)] = h
        with open(file + ".sha256", "w") as f:
            f.write(f"{h}  {file}\n")

# Update manifest
if os.path.exists(manifest_path):
    with open(manifest_path, "r") as f:
        data = json.load(f)
    for release in data.get("releases", []):
        for art in release.get("artifacts", []):
            if isinstance(art, str) and art in hashes:
                data["artifacts"][art]["checksums"]["sha256"] = hashes[art]
    with open(manifest_path, "w") as f:
        json.dump(data, f, indent=2)
