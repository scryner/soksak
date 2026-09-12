#!/usr/bin/env python3
"""Install the optional local alignment runtime without changing system Python."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--venv', type=Path, default=Path.home() / '.soksak' / 'alignment-venv')
    args = parser.parse_args()
    python = args.venv / ('Scripts/python.exe' if os.name == 'nt' else 'bin/python')
    requirements = Path(__file__).with_name('alignment-requirements.txt')
    uv = shutil.which('uv')
    if not python.exists():
        if uv:
            subprocess.run([uv, 'venv', '--python', '3.12', str(args.venv)], check=True)
        else:
            if not (3, 10) <= sys.version_info[:2] < (3, 14):
                parser.error('Use Python 3.10–3.13, or install uv to create a Python 3.12 environment.')
            subprocess.run([sys.executable, '-m', 'venv', str(args.venv)], check=True)
    if uv:
        subprocess.run([uv, 'pip', 'install', '--python', str(python), '-r', str(requirements)], check=True)
    else:
        subprocess.run([str(python), '-m', 'pip', 'install', '-r', str(requirements)], check=True)
    subprocess.run([str(python), '-c', 'from whisperx.alignment import align, load_align_model'], check=True)
    print(f'Alignment runtime ready: {python}')
    print('Language models are downloaded on first alignment and then cached locally.')


if __name__ == '__main__':
    main()
