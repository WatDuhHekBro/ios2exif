# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 1.2.1 - 2025-10-01

### Added

- Better live photos detection, based on the heuristic that the same base filename with different extensions (e.g. `IMG_0369.HEIC` + `IMG_0369.MOV`) are live photo pairs, even if they have different timestamps. The Photos app always shows the HEIC timestamp, not the MOV timestamp.

## 1.2.0 - 2025-09-30

### Added

- Support for live photos, which often have the same base name (e.g. `IMG_0369.HEIC` + `IMG_0369.MOV`)

### Changed

- Less unnecessary warning messages

## 1.1.1 - 2024-05-23

### Added

- Support for older screenshots via XMP metadata

## 1.1.0 - 2024-05-22

### Added

- Support for `mov` files via QuickTime metadata
- Partial support for `mp4` (and some `mov`) files via QuickTime metadata (manual checking required)

### Changed

- Refactored and modularized the codebase

## 1.0.3 - 2023-05-25

### Added

- Program header including version at the beginning of the output
- Which file is displayed if EXIF metadata is present but doesn't contain `DateTimeOriginal`

## 1.0.2 - 2023-05-25

### Added

- The original path with the timestamp in order to make conflict resolution easier

## 1.0.1 - 2023-05-15

### Added

- Lowercase file extensions in renamed files
- Sorted keys in the map (via `BTreeMap`) for more comprehensible output

## 1.0.0 - 2023-05-14

### Added

- Extracting the EXIF `DateTimeOriginal` process
- Bulk renaming
