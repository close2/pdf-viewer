#!/usr/bin/env python3
"""Check the cross-platform gate against the programs the snapshot actually ships.

Cargo supplies binary ownership; CI supplies the shipped population. Calibration uses copies of
those inputs so removing a package or target cannot silently turn a narrower check green.
"""
import copy
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def job(text, name):
    match = re.search(r'^  ' + re.escape(name) + r':\s*$', text, re.M)
    if not match:
        raise ValueError('missing CI job: ' + name)
    tail = text[match.end():]
    following = re.search(r'^  [\w-]+:\s*$', tail, re.M)
    return tail[:following.start()] if following else tail


def matrix(body):
    """The workflow's block-style include rows; reject unsupported or empty shapes."""
    start = body.find('        include:\n')
    end = body.find('\n    runs-on:', start)
    if start < 0 or end < 0:
        raise ValueError('matrix include block absent')
    rows = []
    for line in body[start:end].splitlines()[1:]:
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        if line.startswith('          - '):
            rows.append({})
            field = line[12:]
        elif line.startswith('            ') and rows:
            field = line[12:]
        else:
            raise ValueError('unsupported matrix row: ' + line)
        key, separator, value = field.partition(':')
        if not separator or key in rows[-1]:
            raise ValueError('invalid or duplicate matrix key: ' + field)
        rows[-1][key] = value.strip()
    if not rows:
        raise ValueError('empty matrix')
    return rows


def coverage(snapshot, platforms, packages, metadata):
    portable = [row for row in snapshot if row['os'] != 'ubuntu-24.04']
    shipping = {row['target']: row for row in portable}
    if len(shipping) != len(portable):
        raise ValueError('duplicate snapshot target')
    checked = {row['target'] for row in platforms}
    if not shipping or checked != set(shipping) or len(checked) != len(platforms):
        raise ValueError('platform target coverage differs from non-Linux snapshots')
    for row in platforms:
        if shipping[row['target']]['os'] != row['os']:
            raise ValueError('platform target assigned to a different operating system')
    owners = {}
    for package in metadata['packages']:
        for target in package['targets']:
            if 'bin' in target['kind']:
                owners.setdefault(target['name'], []).append((package['name'], target))
    for triple, row in shipping.items():
        selected = packages[triple]
        if not selected or len(set(selected)) != len(selected):
            raise ValueError('empty or duplicate package selection: ' + triple)
        for binary in row['bins'].split():
            matches = owners.get(binary, [])
            if len(matches) != 1:
                raise ValueError('unknown or ambiguous shipped binary: ' + binary)
            package, target = matches[0]
            if package not in selected:
                raise ValueError('unchecked shipped binary: ' + binary + ' in ' + package)
            if target.get('required-features'):
                raise ValueError('feature-gated shipped binary needs an explicit check: ' + binary)
        # Public ABI libraries are checked independently of the snapshot executable population.
        libraries = {'viewer-ffi'}
        if triple.endswith('apple-darwin'):
            libraries.add('pdf-vfs-ffi')
        if not libraries.issubset(selected):
            raise ValueError('unchecked shared library: ' + triple)


def rejected(call, reason):
    try:
        call()
    except ValueError:
        return
    raise AssertionError('calibration did not reject ' + reason)


def invocation(script, target, selected):
    """A fake Cargo observes argv/flags and fails: the wrapper must preserve its failure."""
    with tempfile.TemporaryDirectory(prefix='platform-check-') as temp:
        path = Path(temp)
        cargo = path / 'cargo'
        cargo.write_text('#!/usr/bin/env python3\nimport json,os,sys\nprint(json.dumps({"args":sys.argv[1:],"flags":os.environ.get("RUSTFLAGS"),"encoded":os.environ.get("CARGO_ENCODED_RUSTFLAGS")}))\nsys.exit(37)\n')
        cargo.chmod(0o755)
        for encoded in (None, '', '-A\x1fwarnings'):
            env = os.environ.copy()
            env['PATH'] = str(path) + os.pathsep + env['PATH']
            env['RUSTFLAGS'] = '-C debuginfo=0 -A warnings'
            env.pop('CARGO_ENCODED_RUSTFLAGS', None)
            if encoded is not None:
                env['CARGO_ENCODED_RUSTFLAGS'] = encoded
            result = subprocess.run(['bash', str(script), target], env=env, text=True, capture_output=True, check=False)
            assert result.returncode == 37, result.stderr
            actual = json.loads(result.stdout)
            expected = ['check', '--all-targets', '--target', target]
            for package in selected:
                expected += ['-p', package]
            assert actual['args'] == expected, actual
            if encoded is None:
                assert actual['flags'] == env['RUSTFLAGS'] + ' -D warnings', actual
            else:
                assert actual['encoded'] == (encoded + '\x1f' if encoded else '') + '-D\x1fwarnings', actual


def main():
    script = ROOT / 'tools/platform-check.sh'
    workflow = (ROOT / '.github/workflows/ci.yml').read_text()
    platform_job = job(workflow, 'platforms')
    snapshot = matrix(job(workflow, 'snapshot'))
    platforms = matrix(platform_job)
    assert '        run: bash tools/platform-check.sh ${{ matrix.target }}' in platform_job.splitlines()
    assert '        run: cargo build --bin quorra --bin pdf-sandbox-worker' in platform_job.splitlines()
    assert '        run: cargo test -p pdf-sandbox -p pdf-render' in platform_job.splitlines()
    assert 'tools/platform-check.sh' in (ROOT / 'doc/verify.md').read_text()
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1', '--locked'], cwd=ROOT, text=True))
    packages = {}
    for row in platforms:
        target = row['target']
        packages[target] = subprocess.check_output(['bash', str(script), '--packages', target], text=True).splitlines()
        invocation(script, target, packages[target])
    coverage(snapshot, platforms, packages, metadata)
    for args in ([], ['unknown-target'], ['--packages', 'unknown-target'], [platforms[0]['target'], 'extra']):
        result = subprocess.run(['bash', str(script), *args], text=True, capture_output=True, check=False)
        assert result.returncode == 64, (args, result)
    missing = copy.deepcopy(packages)
    for selected in missing.values():
        selected.remove('pdf-transform')
    rejected(lambda: coverage(snapshot, platforms, missing, metadata), 'missing transform package')
    rejected(lambda: coverage(snapshot, platforms[1:], packages, metadata), 'missing platform target')
    wrong = copy.deepcopy(platforms)
    wrong[0]['target'] = 'unrecognized-target'
    rejected(lambda: coverage(snapshot, wrong, packages, metadata), 'wrong target')
    unknown = copy.deepcopy(snapshot)
    next(row for row in unknown if row['os'] != 'ubuntu-24.04')['bins'] += ' invented-program'
    rejected(lambda: coverage(unknown, platforms, packages, metadata), 'unknown shipped binary')
    featured = copy.deepcopy(metadata)
    next(target for package in featured['packages'] for target in package['targets'] if target['name'] == 'quorra')['required-features'] = ['unrequested']
    rejected(lambda: coverage(snapshot, platforms, packages, featured), 'feature-gated binary')
    library = copy.deepcopy(packages)
    for selected in library.values():
        selected.remove('viewer-ffi')
    rejected(lambda: coverage(snapshot, platforms, library, metadata), 'missing shared library')
    print('platform coverage: shipped binaries and libraries covered; six calibrated omissions rejected; invocation/flags/failure propagation passed')


if __name__ == '__main__':
    main()
