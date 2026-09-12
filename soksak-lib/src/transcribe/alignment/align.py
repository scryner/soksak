"""Local, alignment-only WhisperX bridge. Never changes or re-transcribes text."""
import argparse
import contextlib
import json
import math
import os
from pathlib import Path
import sys
import time
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


def select_device(torch, requested):
    if requested not in ('auto', 'cpu', 'mps', 'cuda'):
        raise ValueError(f'Unknown alignment device: {requested}')
    if requested == 'cpu':
        return 'cpu'
    if requested in ('auto', 'cuda') and torch.cuda.is_available():
        return 'cuda'
    if requested in ('auto', 'mps') and torch.backends.mps.is_available():
        return 'mps'
    if requested != 'auto':
        raise RuntimeError(f'Requested alignment device is unavailable: {requested}')
    return 'cpu'


def write_progress(request, phase, completed):
    if request.get('progress'):
        path = Path(request['progress'])
        temporary = path.with_suffix('.tmp')
        temporary.write_text(json.dumps({'phase': phase, 'completed': completed}), encoding='utf-8')
        temporary.replace(path)


def align_with_fallback(align, transcript, model, metadata, audio, device, requested_device, torch):
    try:
        return align(transcript, model, metadata, audio, device, return_char_alignments=True), device, None
    except (RuntimeError, NotImplementedError) as error:
        if requested_device != 'auto' or device == 'cpu':
            raise
        reason = f'{device} inference failed: {type(error).__name__}: {error}'[:500]
        model.to('cpu')
        if torch.backends.mps.is_available():
            torch.mps.empty_cache()
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
        result = align(transcript, model, metadata, audio, 'cpu', return_char_alignments=True)
        return result, 'cpu', reason


def run(request):
    started = time.perf_counter()
    write_progress(request, 'loading_model', 0)
    import numpy as np
    import torch
    from whisperx.alignment import align, load_align_model, DEFAULT_ALIGN_MODELS_TORCH, DEFAULT_ALIGN_MODELS_HF
    torch.set_num_threads(min(4, os.cpu_count() or 1))
    requested_device = request.get('device', 'auto')
    device = select_device(torch, requested_device)
    # Read the prepared WAV directly: another ffmpeg decode could reset its timeline.
    with wave.open(request['audio'], 'rb') as f:
        if (f.getframerate(), f.getnchannels(), f.getsampwidth()) != (16000, 1, 2):
            raise ValueError('Expected 16kHz mono signed 16-bit PCM')
        audio = np.frombuffer(f.readframes(f.getnframes()), dtype='<i2').astype(np.float32) / 32768.0
    load_started = time.perf_counter()
    # Load once on CPU, then transfer. This permits an explicit, reported CPU retry
    # when an accelerator is unavailable at runtime (rather than silent per-op fallback).
    try:
        # Cached Hugging Face models should not wait for network metadata on every video.
        model, metadata = load_align_model(request['language'], 'cpu', model_name=request.get('model'), model_cache_only=True)
    except (ValueError, OSError):
        model, metadata = load_align_model(request['language'], 'cpu', model_name=request.get('model'))
    fallback_reason = None
    try:
        model.to(device).eval()
    except (RuntimeError, NotImplementedError) as error:
        if requested_device != 'auto' or device == 'cpu':
            raise
        fallback_reason = f'{device} model transfer failed: {type(error).__name__}: {error}'[:500]
        device = 'cpu'
        model.to(device).eval()
    model_load_seconds = time.perf_counter() - load_started
    processing_started = time.perf_counter()
    write_progress(request, 'aligning', 0)
    candidates = []
    for completed, item in enumerate(request['segments'], 1):
        row = {'id': item['id']}
        try:
            # Bound inference memory per window and reject huge uncertain windows.
            if item['search_end'] - item['search_start'] > 90:
                row['reason'] = 'search_window_too_long'
            else:
                transcript = [{'start': item['search_start'], 'end': item['search_end'], 'text': item['text']}]
                result, device, reason = align_with_fallback(
                    align, transcript, model, metadata, audio, device, requested_device, torch)
                fallback_reason = reason or fallback_reason
                row.update(candidate_from_result(result, item['text']))
        except Exception as error:
            row['reason'] = f'alignment_error:{type(error).__name__}'
        candidates.append(row)
        write_progress(request, 'aligning', completed)
    model_name = request.get('model') or DEFAULT_ALIGN_MODELS_TORCH.get(request['language']) or DEFAULT_ALIGN_MODELS_HF.get(request['language'])
    return {'candidates': candidates, 'runtime': {
        'device': device, 'model': model_name,
        'model_load_seconds': model_load_seconds,
        'processing_seconds': time.perf_counter() - processing_started,
        'total_seconds': time.perf_counter() - started,
        'fallback_reason': fallback_reason,
    }}


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
