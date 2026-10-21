#!/usr/bin/env bash
# Compile the Windows host's C# with the real SDK references on any SDK host.
# XAML generation and packaging still belong to the Windows CI job.
set -euo pipefail
check_root=${DOTNET_CHECK_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}
check_sdk=
for candidate in "${DOTNET_ROOT:-}/dotnet" /home/AI/dotnet-check/dotnet/dotnet; do
    if [ -x "$candidate" ]; then check_sdk=$candidate; break; fi
done
if [ -z "$check_sdk" ]; then check_sdk=$(command -v dotnet || true); fi
if [ -z "$check_sdk" ]; then
    echo 'dotnet-check: no .NET SDK found (DOTNET_ROOT, saved SDK, or PATH)' >&2
    exit 69
fi
if [ "${1:-}" = --probe ]; then printf '%s\n' "$check_sdk"; exit 0; fi
if [ "$#" -ne 0 ]; then echo 'usage: tools/dotnet-check.sh [--probe]' >&2; exit 64; fi
check_work=$(mktemp -d "${TMPDIR:-/tmp}/quorra-dotnet-check.XXXXXXXX")
trap 'rm -rf -- "$check_work"' EXIT
export DOTNET_ROOT
DOTNET_ROOT=$(dirname "$check_sdk")
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
export DOTNET_CLI_HOME="$check_work/home"
if [ -z "${NUGET_PACKAGES:-}" ] && [ -d /home/AI/dotnet-check/cs/packages ]; then
    export NUGET_PACKAGES=/home/AI/dotnet-check/cs/packages
fi
PYTHONDONTWRITEBYTECODE=1 python3 -I -B - "$check_root" "$check_work" <<'PY'
from pathlib import Path
import copy
import sys
import xml.etree.ElementTree as ET

source = Path(sys.argv[1]) / 'hosts/windows'
work = Path(sys.argv[2])
properties = ('TargetFramework', 'TargetPlatformMinVersion', 'RootNamespace',
              'ImplicitUsings', 'Nullable', 'AllowUnsafeBlocks', 'TreatWarningsAsErrors',
              'Platforms', 'RuntimeIdentifier')
for name in ('Quorra.Interop', 'Quorra.App'):
    directory = source / name
    original = ET.parse(directory / (name + '.csproj')).getroot()
    project = ET.Element('Project', Sdk='Microsoft.NET.Sdk')
    group = ET.SubElement(project, 'PropertyGroup')
    for key in properties:
        value = original.find('.//' + key)
        if value is not None:
            group.append(copy.deepcopy(value))
    for key, value in {'AssemblyName': name, 'OutputType': 'Library',
                       'EnableDefaultCompileItems': 'false', 'EnableWindowsTargeting': 'true',
                       'WindowsPackageType': 'None'}.items():
        ET.SubElement(group, key).text = value
    items = ET.SubElement(project, 'ItemGroup')
    for item in original.findall('.//PackageReference') + original.findall('.//InternalsVisibleTo'):
        items.append(copy.deepcopy(item))
    for path in sorted(directory.rglob('*.cs')):
        if not {'bin', 'obj'}.intersection(path.relative_to(directory).parts):
            ET.SubElement(items, 'Compile', Include=str(path))
    destination = work / name
    destination.mkdir()
    if name == 'Quorra.App':
        ET.SubElement(items, 'ProjectReference', Include='../Quorra.Interop/Check.csproj')
        stubs = []
        xaml = '{http://schemas.microsoft.com/winfx/2006/xaml}'
        for path in sorted(directory.glob('*.xaml')):
            root = ET.parse(path).getroot()
            namespace, _, class_name = root.attrib[xaml + 'Class'].rpartition('.')
            stubs += [f'namespace {namespace} {{', f'public partial class {class_name} {{']
            for element in root.iter():
                field = element.get(xaml + 'Name')
                if not field:
                    continue
                uri, _, kind = element.tag[1:].partition('}')
                type_namespace = uri[6:] if uri.startswith('using:') else 'Microsoft.UI.Xaml.Controls'
                stubs.append(f'internal {type_namespace}.{kind} {field} = null!;')
            stubs += ['private void InitializeComponent() {}', '}', '}']
        (destination / 'XamlStubs.cs').write_text('\n'.join(stubs) + '\n')
        ET.SubElement(items, 'Compile', Include='XamlStubs.cs')
    ET.ElementTree(project).write(destination / 'Check.csproj', encoding='utf-8', xml_declaration=True)
PY
"$check_sdk" build "$check_work/Quorra.App/Check.csproj" --nologo \
    -p:AppxGeneratePriEnabled=false -p:GenerateLibraryLayout=false
