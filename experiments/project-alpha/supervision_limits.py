"""Local fixed budgets for a long-lived generated alpha; never authority grants."""
import math
import os
from pathlib import Path
import stat

from project_schema import MAX_CONTENT, path as source_path

MAX_FILES = 128
MAX_DIRECTORIES = 128
WINDOW_SECONDS = 60
HTTP_PER_WINDOW = 512
STARTS_PER_WINDOW = 16
MAX_JOBS = 16
MAX_SEQUENCE = (1 << 63) - 1


class WindowBudget:
    def __init__(self, limit, seconds, now):
        if (type(limit) is not int or not 1 <= limit <= 512
                or type(seconds) is not int or not 1 <= seconds <= 60
                or type(now) not in (int, float) or not math.isfinite(now)):
            raise ValueError('supervision.invalid_budget')
        self.limit, self.seconds = limit, seconds
        self.started = self.last = now
        self.used = 0
        self.failed = False

    def take(self, now):
        if (self.failed or type(now) not in (int, float)
                or not math.isfinite(now) or now < self.last):
            self.failed = True
            return False
        self.last = now
        if now - self.started >= self.seconds:
            self.started, self.used = now, 0
        if self.used >= self.limit:
            return False
        self.used += 1
        return True


class ProjectFiles:
    def __init__(self, paths=(), directories=()):
        self.paths = set()
        self.directories = set()
        for path in paths:
            self.reserve(path)
        for path in directories:
            self.reserve_directory(path)

    def reserve_directory(self, path):
        key = source_path(path).lower()
        if key not in self.directories and len(self.directories) >= MAX_DIRECTORIES:
            raise ValueError('project.directory_capacity')
        self.directories.add(key)

    def reserve(self, path):
        # Generated schema is ASCII; Windows aliases share a conservative slot.
        key = source_path(path, file=True).lower()
        if key not in self.paths and len(self.paths) >= MAX_FILES:
            raise ValueError('project.file_capacity')
        self.paths.add(key)

    @classmethod
    def inspect(cls, project):
        # Operator startup only, before remote admission. Do not read contents.
        root = Path(project) / 'src'
        pending, paths, directories, names = [root], [], 0, []
        while pending:
            directory = pending.pop()
            info = directory.lstat()
            if (not stat.S_ISDIR(info.st_mode) or stat.S_ISLNK(info.st_mode)
                    or getattr(info, 'st_file_attributes', 0) & 0x400):
                raise ValueError('project.inventory_invalid')
            directories += 1
            names.append(directory.relative_to(project).as_posix())
            if directories > MAX_DIRECTORIES:
                raise ValueError('project.directory_capacity')
            with os.scandir(directory) as entries:
                for entry in entries:
                    info = entry.stat(follow_symlinks=False)
                    if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & 0x400:
                        raise ValueError('project.inventory_invalid')
                    relative = Path(entry.path).relative_to(project).as_posix()
                    if stat.S_ISDIR(info.st_mode):
                        source_path(relative)
                        if len(pending) + directories >= MAX_DIRECTORIES:
                            raise ValueError('project.directory_capacity')
                        pending.append(Path(entry.path))
                    elif stat.S_ISREG(info.st_mode) and info.st_size <= MAX_CONTENT:
                        if len(paths) >= MAX_FILES:
                            raise ValueError('project.file_capacity')
                        paths.append(source_path(relative, file=True))
                    else:
                        raise ValueError('project.inventory_invalid')
        return cls(paths, names)
