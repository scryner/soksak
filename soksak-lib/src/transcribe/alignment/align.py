"""Local, alignment-only WhisperX bridge. Never changes or re-transcribes text."""
import argparse
import contextlib
import json
import math
import os
from pathlib import Path
import sys
import wave


def candidate_from_result(result, text):
    # Keep punctuation/whitespace in the caller's original text. Require both
    # spoken edges and enough interior characters to actually have acoustic scores.
    chars = [c for s in result['segments'] for c in (s.get('chars') or [])
             if c.get('char', '').isalnum()]
    expected = ''.join(c.lower() for c in text if c.isalnum())
    actual = ''.join(c['char'].lower() for c in chars)
    if not expected or actual != expected:
        return {'reason': 'character_mismatch'}
    def scored(c):
        return all(isinstance(c.get(k), (float, int)) and math.isfinite(c[k])
                   for k in ('start', 'end', 'score')) and c['end'] > c['start']
    if not scored(chars[0]) or not scored(chars[-1]):
        return {'reason': 'unaligned_edge'}
    usable = [c for c in chars if scored(c)]
    return {
        'start': chars[0]['start'], 'end': chars[-1]['end'],
        'score': sum(c['score'] for c in usable) / len(usable),
        'coverage': len(usable) / len(chars),
    }


def run(request):
    import numpy as np
    import torch
    from whisperx.alignment import align, load_align_model
    torch.set_num_threads(min(4, os.cpu_count() or 1))
    # Read the prepared WAV directly: another ffmpeg decode could reset its timeline.
    with wave.open(request['audio'], 'rb') as f:
        if (f.getframerate(), f.getnchannels(), f.getsampwidth()) != (16000, 1, 2):
            raise ValueError('Expected 16kHz mono signed 16-bit PCM')
        audio = np.frombuffer(f.readframes(f.getnframes()), dtype='<i2').astype(np.float32) / 32768.0
    model, metadata = load_align_model(request['language'], 'cpu', model_name=request.get('model'))
    candidates = []
    for item in request['segments']:
        row = {'id': item['id']}
        try:
            # Bound inference memory per window and reject huge uncertain windows.
            if item['search_end'] - item['search_start'] > 90:
                row['reason'] = 'search_window_too_long'
            else:
                result = align([{'start': item['search_start'], 'end': item['search_end'], 'text': item['text']}],
                               model, metadata, audio, 'cpu', return_char_alignments=True)
                row.update(candidate_from_result(result, item['text']))
        except Exception as error:
            row['reason'] = f'alignment_error:{type(error).__name__}'
        candidates.append(row)
    return {'candidates': candidates}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('request', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    request = json.loads(args.request.read_text(encoding='utf-8'))
    # Model/library logging is separate from the JSON protocol.
    with contextlib.redirect_stdout(sys.stderr):
        response = run(request)
    args.output.write_text(json.dumps(response, ensure_ascii=False, allow_nan=False), encoding='utf-8')


if __name__ == '__main__':
    main()
