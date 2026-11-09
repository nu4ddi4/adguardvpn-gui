"""배포 RPM에 포함할 Rust 의존성의 라이선스·고지 파일을 모읍니다."""
from pathlib import Path
import shutil

source = Path('build/source')
output = source / 'licenses'
for package in (source / 'vendor').iterdir():
    if not package.is_dir():
        continue
    for item in package.iterdir():
        if item.is_file() and item.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE', 'COPYRIGHT')):
            target = output / package.name / item.name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(item, target)
