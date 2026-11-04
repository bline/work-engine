#!/usr/bin/env python3
"""Fixture board adapter. Existing board owner supplies revision and cursor semantics.

Only service configuration chooses repository/reader. Raw row slices come from
that exact immutable durable revision, never from reserialized wait projections.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import resource

MAX_SNAPSHOT = 8 * 1024 * 1024


MAX_HISTORY_REVISIONS = 128


class SourceHistoryError(ValueError):
    pass


def snapshot(owner, value):
    if len(value.payload) > MAX_SNAPSHOT:
        raise ValueError('source_oversized')
    text = value.payload.decode('utf-8')
    board = owner.decode(value.payload)
    # The owner codec writes sorted object keys. Locate the top-level messages
    # value using a JSON decoder, without searching for an arbitrary text token.
    decoder = json.JSONDecoder()
    pos = text.index('{') + 1
    raw_rows = []
    while True:
        while text[pos].isspace() or text[pos] == ',': pos += 1
        if text[pos] == '}': break
        key, pos = decoder.raw_decode(text, pos)
        while text[pos].isspace(): pos += 1
        if text[pos] != ':': raise ValueError('malformed')
        pos += 1
        while text[pos].isspace(): pos += 1
        if key == 'messages':
            if text[pos] != '[': raise ValueError('malformed')
            pos += 1
            while True:
                while text[pos].isspace() or text[pos] == ',': pos += 1
                if text[pos] == ']':
                    pos += 1
                    break
                start = pos
                row, pos = decoder.raw_decode(text, pos)
                raw_rows.append((row, text[start:pos]))
        else:
            _, pos = decoder.raw_decode(text, pos)
    if len(raw_rows) > 10000 or [r['sequence'] for r, _ in raw_rows] != list(range(1, board['next_sequence'])):
        raise ValueError('source_gap_or_backlog')
    return board, raw_rows


def verify_history(owner, store, current, raw_rows, since, previous_revision):
    """Check the committed observation's history before selecting new rows.

    Prefix comparison includes reserved rows omitted by wait_board. History
    reachability consumes the existing durable owner's predecessor links only;
    neither a reused path nor a caught-up scalar cursor establishes continuity.
    """
    if previous_revision is None:
        if since != 0:
            raise SourceHistoryError('source_history_missing_baseline')
        return
    try:
        prior = store.read_revision(owner.BOARD_KEY, previous_revision)
        board, prior_rows = snapshot(owner, prior)
        if board['next_sequence'] - 1 < since:
            raise SourceHistoryError('source_history_incompatible_cursor')
        prefix = lambda rows: [raw for row, raw in rows if row['sequence'] <= since]
        if prefix(prior_rows) != prefix(raw_rows):
            raise SourceHistoryError('source_history_consumed_prefix_conflict')
        seen = set()
        value = current
        for _ in range(MAX_HISTORY_REVISIONS):
            if value.revision == previous_revision:
                return
            if value.revision in seen or value.predecessor_revision is None:
                raise SourceHistoryError('source_history_detached_or_cyclic')
            seen.add(value.revision)
            value = store.read_revision(owner.BOARD_KEY, value.predecessor_revision)
        raise SourceHistoryError('source_history_verification_bound')
    except SourceHistoryError:
        raise
    except (ValueError, OSError) as error:
        raise SourceHistoryError('source_history_unavailable_or_invalid') from error


def observe(reader, repository, since, previous_revision):
    resource.setrlimit(resource.RLIMIT_AS, (256 * 1024 * 1024, 256 * 1024 * 1024))
    resource.setrlimit(resource.RLIMIT_CPU, (10, 10))
    sys.path.insert(0, str(Path(reader).resolve().parent))
    spec = importlib.util.spec_from_file_location('delivery_board_owner', reader)
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    result = owner.wait_board(Path(repository), since=since, limit=500, timeout_seconds=0)
    if result['revision'] is None:
        raise ValueError('source_unavailable:no durable board revision')
    store = owner.GitRefDurableStateStore(Path(repository), owner.NAMESPACE)
    value = store.read_revision(owner.BOARD_KEY, result['revision'])
    board, raw_rows = snapshot(owner, value)
    if board['next_sequence'] - 1 < since:
        raise SourceHistoryError('source_history_gap_or_rewind')
    verify_history(owner, store, value, raw_rows, since, previous_revision)
    selected = {r['sequence'] for r in result['messages']}
    common = subprocess.check_output(['git', '-C', repository, 'rev-parse', '--path-format=absolute', '--git-common-dir'], text=True).strip()
    return {'schema': 'board-observation-v1', 'repository': str(Path(common).resolve()),
            'source_key': owner.BOARD_KEY, 'revision': value.revision,
            'previous_revision': previous_revision,
            'epoch': 'git-ref-v1:' + owner.BOARD_KEY, 'since': since,
            'next_since': result['next_since'], 'has_more': result['has_more'],
            'rows': [raw for row, raw in raw_rows if row['sequence'] in selected]}


if __name__ == '__main__':
    try:
        print(json.dumps(observe(sys.argv[1], sys.argv[2], int(sys.argv[3]),
                                 None if sys.argv[4] == "-" else sys.argv[4]), ensure_ascii=False))
    except SourceHistoryError as error:
        print(str(error), file=sys.stderr)
        sys.exit(17)
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
