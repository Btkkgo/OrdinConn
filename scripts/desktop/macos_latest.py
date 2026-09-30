#!/usr/bin/env python3
"""Build one local macOS acceptance bundle; fail closed before removing stale apps.

No settings, databases, credentials, source files, evidence, or global LS resets.
Only this module's reviewed stale .app paths may reach remove_verified().
"""
import argparse
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import time

IDENTIFIER = 'ai.ordinconn.desktop'
EXECUTABLE = 'ordinconn-desktop'
REPOSITORY = 'https://github.com/Btkkgo/OrdinConn'
LSREGISTER = '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister'
PROTECTED_NAMES = {'.git', '.env', 'docs', 'artifacts', 'evidence', 'screenshots', 'devlog', 'keychains'}
PROTECTED_SUFFIXES = {'.sqlite', '.sqlite3', '.db', '.sqlite3-wal', '.sqlite3-shm', '.keychain', '.keychain-db', '.jpeg', '.jpg', '.rs', '.tsx', '.ts', '.py', '.md', '.toml'}

class Blocked(RuntimeError):
    pass

def run(args, cwd=None):
    result = subprocess.run([str(x) for x in args], cwd=cwd, text=True, capture_output=True)
    if result.returncode:
        raise Blocked(f'Command failed: {Path(str(args[0])).name} (exit {result.returncode})')
    return result.stdout

def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''): digest.update(block)
    return digest.hexdigest()

def no_symlink(path):
    if any(p.is_symlink() for p in [path, *path.parents]):
        raise Blocked(f'Symlink path preserved: {path}')

def bundle_digest(path):
    no_symlink(path)
    if not path.is_dir() or path.suffix != '.app': raise Blocked(f'Not an app directory: {path}')
    if {p.name for p in path.iterdir()} != {'Contents'}:
        raise Blocked(f'Unexpected top-level bundle payload preserved: {path}')
    digest = hashlib.sha256()
    for item in sorted(path.rglob('*')):
        relative = item.relative_to(path)
        if item.is_symlink(): raise Blocked(f'Bundle with symlinks preserved: {path}')
        if any(part.lower() in PROTECTED_NAMES for part in relative.parts) or item.suffix.lower() in PROTECTED_SUFFIXES:
            raise Blocked(f'Bundle containing protected data preserved: {path}')
        if not item.is_dir() and not item.is_file(): raise Blocked(f'Special bundle file preserved: {path}')
        digest.update(str(relative).encode() + b'\0')
        digest.update(str(item.stat().st_mode & 0o777).encode() + b'\0')
        if item.is_file(): digest.update(sha(item).encode())
    return digest.hexdigest()

def bundle_info(path):
    digest = bundle_digest(path)
    try:
        info = plistlib.loads((path / 'Contents/Info.plist').read_bytes())
    except Exception as error:
        raise Blocked(f'Unreadable app identity preserved: {path}') from error
    expected = {'CFBundleIdentifier': IDENTIFIER, 'CFBundleExecutable': EXECUTABLE, 'CFBundlePackageType': 'APPL'}
    if any(info.get(k) != v for k, v in expected.items()): raise Blocked(f'Unrecognized app identity preserved: {path}')
    exe = path / 'Contents/MacOS' / EXECUTABLE
    if not exe.is_file() or not os.access(exe, os.X_OK): raise Blocked(f'No runnable executable: {path}')
    with exe.open('rb') as stream: magic = stream.read(4)
    if magic not in [b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xca\xfe\xba\xbe', b'\xca\xfe\xba\xbf']:
        raise Blocked(f'Not a native Mach-O executable: {path}')
    return {'path': str(path), 'identifier': info['CFBundleIdentifier'],
            'version': info.get('CFBundleShortVersionString'), 'build': info.get('CFBundleVersion'),
            'modified_ns': max(p.stat().st_mtime_ns for p in [path, *path.rglob('*')]),
            'bundle_digest': digest, 'executable_digest': sha(exe)}

def build_locations(root):
    return [base / profile / 'bundle/macos/OrdinConn.app'
            for base in [root / 'target', root / 'apps/desktop/src-tauri/target']
            for profile in ['release', 'debug']]

def approve_stale(path, root, known_hashes, latest_ns):
    path, root = Path(path).absolute(), Path(root).absolute()
    info = bundle_info(path)
    if info['modified_ns'] > latest_ns: raise Blocked(f'Newer or concurrently changed app preserved: {path}')
    installed = {Path('/Applications/OrdinConn.app'), Path.home() / 'Applications/OrdinConn.app'}
    if path not in build_locations(root) and path not in installed:
        raise Blocked(f'Unrecognized app location preserved: {path}')
    if path.is_relative_to(root) and run(['git', 'ls-files', '--', str(path.relative_to(root))], root).strip():
        raise Blocked(f'Git-tracked bundle preserved: {path}')
    if info['bundle_digest'] in known_hashes: return info['bundle_digest']
    if path in build_locations(root):
        # Bootstrap only canonical project outputs whose bytes match their Cargo binary.
        allowed = {'Contents/Info.plist', f'Contents/MacOS/{EXECUTABLE}', 'Contents/Resources/OrdinConn.icns'}
        if any(str(p.relative_to(path)) not in allowed for p in path.rglob('*') if p.is_file()):
            raise Blocked(f'Unrecognized resource in unrecorded bundle; preserved: {path}')
        raw = path.parents[2] / EXECUTABLE
        no_symlink(raw)
        if raw.is_file() and sha(raw) == info['executable_digest']: return info['bundle_digest']
    raise Blocked(f'Build provenance unavailable; app preserved: {path}')

def remove_verified(path, expected_digest):
    if bundle_digest(path) != expected_digest: raise Blocked(f'App changed since review; preserved: {path}')
    if not shutil.rmtree.avoids_symlink_attacks: raise Blocked('Symlink-safe directory removal unavailable')
    shutil.rmtree(path)

def ls_paths():
    dump = run([LSREGISTER, '-dump'])
    paths = set()
    for block in re.split(r'\n-{20,}\n', dump):
        match = re.search(r'^path:\s+(.+?)(?: \(0x[0-9a-f]+\))?$', block, re.M)
        if not match: continue
        path = Path(match.group(1))
        if path.name == 'OrdinConn.app' or re.search(r'^identifier:\s+' + re.escape(IDENTIFIER) + r'\s*$', block, re.M):
            paths.add(path)
    return paths

def spotlight_paths():
    query = f'kMDItemCFBundleIdentifier == "{IDENTIFIER}" || kMDItemFSName == "OrdinConn.app"'
    return {Path(p) for p in run(['/usr/bin/mdfind', query]).splitlines() if p}

def discover(root):
    paths = ls_paths() | spotlight_paths()
    for directory in [Path('/Applications'), Path.home() / 'Applications', root]:
        if not directory.is_dir(): continue
        for current, dirs, _ in os.walk(directory, followlinks=False):
            dirs[:] = [d for d in dirs if d not in ['.git', 'node_modules', '.Trash']]
            for name in dirs:
                p = Path(current) / name
                if p.is_symlink() and name == 'OrdinConn.app': paths.add(p)
            path = Path(current)
            if path.suffix != '.app': continue
            try: identifier = plistlib.loads((path / 'Contents/Info.plist').read_bytes()).get('CFBundleIdentifier')
            except Exception: identifier = None
            if path.name == 'OrdinConn.app' or identifier == IDENTIFIER: paths.add(path)
            dirs[:] = []
    return paths

def assert_not_running(paths):
    executables = {str(path / 'Contents/MacOS' / name) for path in paths for name in [EXECUTABLE, 'OrdinConn']}
    running = {line.strip() for line in run(['/bin/ps', '-axo', 'comm=']).splitlines()}
    if executables & running: raise Blocked('Quit OrdinConn before replacing or removing its application bundle')

def source_state(root):
    head = run(['git', 'rev-parse', 'HEAD'], root).strip()
    files = run(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], root).split('\0')
    digest = hashlib.sha256()
    for name in sorted(set(files)):
        if not (name in ['Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json'] or name.startswith(('apps/', 'crates/', 'packages/', 'scripts/desktop/'))): continue
        path = root / name
        if not path.exists(): digest.update(name.encode() + b'\0DELETED'); continue
        no_symlink(path)
        if path.is_file(): digest.update(name.encode() + b'\0' + sha(path).encode())
    return {'head': head, 'source_fingerprint': digest.hexdigest(),
            'dirty': bool(run(['git', 'status', '--porcelain'], root).strip())}

def receipt_directory(root): return root / 'target/ordinconn-build-receipts'

def assert_build_unfrozen(root):
    """A final acceptance binary is immutable until its owner completes both gates."""
    marker = root / 'target/final-m3-gate/FINAL_M3_BINARY_FROZEN.json'
    if not marker.exists() and not marker.is_symlink(): return
    no_symlink(marker)
    try:
        if not marker.is_file() or marker.stat().st_size > 65_536:
            raise ValueError('Invalid freeze marker')
        state = json.loads(marker.read_text())
        hashes = ['executable_sha256', 'bundle_fingerprint', 'source_fingerprint', 'dirty_worktree_fingerprint']
        valid = (isinstance(state, dict) and state.get('schema_version') == 1
                 and state.get('state') == 'FROZEN'
                 and state.get('app_path') in {str(p.relative_to(root)) for p in build_locations(root)}
                 and isinstance(state.get('source_head'), str)
                 and re.fullmatch('[0-9a-f]{40}', state['source_head'])
                 and all(isinstance(state.get(k), str) and re.fullmatch('[0-9a-f]{64}', state[k]) for k in hashes)
                 and all(isinstance(state.get(k), str) and state[k] for k in ['built_at_utc', 'frozen_at_utc', 'signing_identity']))
        if not valid: raise ValueError('Unknown freeze state')
    except (OSError, ValueError, TypeError) as error:
        raise Blocked('Unknown final acceptance freeze marker; do not rebuild') from error
    raise Blocked('FINAL_M3_BINARY_FROZEN: do not rebuild before Planner 5/5 and Android Final Gate finish')

def known_receipts(root):
    result = set()
    directory = receipt_directory(root)
    if not directory.exists(): return result
    no_symlink(directory)
    for path in directory.glob('*.json'):
        no_symlink(path)
        receipt = json.loads(path.read_text())
        digest = receipt.get('bundle_digest', '')
        if receipt.get('repository') != REPOSITORY or not re.fullmatch('[0-9a-f]{64}', digest) or not re.fullmatch(re.escape(digest) + r'(?:\.[0-9]+)?', path.stem):
            raise Blocked('Invalid build receipt; do not delete any bundles')
        result.add(digest)
    return result

def verify_single(root, keep):
    run([LSREGISTER, '-f', keep])
    run(['/usr/bin/mdimport', keep])
    # Spotlight removal/import is asynchronous. Never report success from registration alone.
    for _ in range(15):
        registered = {p for p in ls_paths() if p.exists()}
        indexed = spotlight_paths()
        actual = {p for p in discover(root) if p.exists() or p.is_symlink()}
        if registered == indexed == actual == {keep}:
            return {'launch_services': 'PASS: one valid target', 'spotlight': 'PASS: one indexed target'}
        time.sleep(1)
    raise Blocked('Registration/index verification incomplete; no further deletion attempted')

def unregister_missing(paths):
    registered = ls_paths()
    for path in sorted(paths & registered):
        if not path.exists() and not path.is_symlink():
            # A removed bundle may already have been unregistered in the removal loop.
            # Repeating -u for an absent registration returns an error on macOS.
            run([LSREGISTER, '-u', path])

def build(root, debug):
    assert_build_unfrozen(root)
    if os.environ.get('CARGO_TARGET_DIR') or os.environ.get('CARGO_BUILD_TARGET'):
        raise Blocked('Custom Cargo output/target requires separate provenance review')
    config = json.loads((root / 'apps/desktop/src-tauri/tauri.conf.json').read_text())
    if config.get('identifier') != IDENTIFIER or config.get('bundle', {}).get('targets') != ['app']:
        raise Blocked('Unexpected bundler configuration; no cleanup attempted')
    profile = 'debug' if debug else 'release'
    keep = root / f'target/{profile}/bundle/macos/OrdinConn.app'
    known = known_receipts(root)
    previous = discover(root)
    assert_not_running(previous)
    initial = {}
    # Complete all ownership checks before building over a previous bundle.
    for path in sorted(previous):
        if path.exists() or path.is_symlink():
            digest = approve_stale(path, root, known, time.time_ns())
            if digest in known:
                run(['/usr/bin/codesign', '--verify', '--deep', '--strict', path])
            else:
                # Legacy Tauri app wrappers are unsealed, but their matching Cargo executable is signed.
                run(['/usr/bin/codesign', '--verify', path.parents[2] / EXECUTABLE])
            initial[path] = digest
    before = source_state(root)
    executable = root / 'node_modules/.bin/tauri'
    args = [str(executable), 'build'] + (['--debug'] if debug else [])
    result = subprocess.run(args, cwd=root / 'apps/desktop')
    if result.returncode: raise Blocked('Build failed; previous alternate app bundles were preserved')
    if source_state(root) != before: raise Blocked('Source changed during build; no stale bundle removed')
    bundle_info(keep)
    signing = subprocess.run(['/usr/bin/codesign', '-dv', str(keep)], capture_output=True, text=True)
    if signing.returncode or 'Signature=adhoc' not in signing.stderr:
        raise Blocked('Expected a local ad-hoc build; preserving all bundles')
    # Tauri's linker signature does not seal the app resources; seal the local acceptance bundle.
    run(['/usr/bin/codesign', '--force', '--sign', '-', keep])
    run(['/usr/bin/codesign', '--verify', '--deep', '--strict', keep])
    latest = bundle_info(keep)
    receipt = dict(before, **latest, repository=REPOSITORY,
                   built_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
    # Persist sanitized provenance; never store machine home paths or credential data.
    receipt['path'] = str(keep.relative_to(root))
    directory = receipt_directory(root)
    directory.mkdir(parents=True, exist_ok=True)
    no_symlink(directory)
    record = directory / f"{latest['bundle_digest']}.{time.time_ns()}.json"
    no_symlink(record)
    with record.open('x') as output:
        output.write(json.dumps(receipt, indent=2) + '\n')
    current = discover(root)
    assert_not_running(current)
    candidates = []
    for path in sorted(current):
        if path == keep or not (path.exists() or path.is_symlink()): continue
        digest = approve_stale(path, root, known, latest['modified_ns'])
        if initial.get(path) != digest: raise Blocked(f'App appeared or changed during build; preserved: {path}')
        candidates.append((path, digest))
    # All candidates validated before the first removal. No broad rm, reset, or cache purge.
    for path, digest in candidates:
        run([LSREGISTER, '-u', path])
        remove_verified(path, digest)
        print(f'Removed verified stale bundle: {path}', flush=True)
    unregister_missing((previous | current) - {keep})
    results = verify_single(root, keep)
    print(json.dumps(dict(receipt, **results), indent=2), flush=True)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--debug', action='store_true', help='Build and keep debug instead of release')
    parser.add_argument('--audit', action='store_true', help='Read-only inventory; never build or remove')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    if sys.platform != 'darwin':
        if args.audit: raise Blocked('macOS inventory is unavailable on this platform')
        command = [str(root / 'node_modules/.bin/tauri'), 'build'] + (['--debug'] if args.debug else [])
        return subprocess.call(command, cwd=root / 'apps/desktop')
    no_symlink(root)
    if run(['git', 'rev-parse', '--show-toplevel'], root).strip() != str(root): raise Blocked('Wrong repository root')
    if args.audit:
        for path in sorted(discover(root)):
            if path.exists() or path.is_symlink():
                try: print(json.dumps(bundle_info(path)))
                except Blocked as error: print(f'UNKNOWN: {error}')
            else: print(f'Stale registration (no file): {path}')
        return 0
    target = root / 'target'
    target.mkdir(exist_ok=True)
    no_symlink(target)
    lock_path = target / '.ordinconn-bundle.lock'
    no_symlink(lock_path)
    with lock_path.open('a') as lock:
        try: fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError: raise Blocked('Another acceptance build is active')
        build(root, args.debug)
    return 0

if __name__ == '__main__':
    try: sys.exit(main())
    except (Blocked, OSError, ValueError) as error:
        print(f'BLOCKED: {error}', file=sys.stderr)
        sys.exit(1)
