# DriveAnalizer - Release Notes

## 🚀 Initial Release v0.1.0

A lightweight, high-performance disk I/O and system monitoring tool for Windows and Linux.

**Key Features:**
- Real-Time Monitoring: Track disk read/write speeds with millisecond precision
- High-Performance Charts: Ultra-fast visualization of thousands of data points using uPlot
- Efficient Storage: SQLite database with smart buffering to preserve system performance
- Process Tracking: Monitor which processes consume the most system resources
- Lightweight: Minimal RAM and CPU overhead - significantly faster than Electron alternatives
- Cross-Platform: Works seamlessly on Windows and Linux

**Technology Stack:**
Tauri v2 + Rust backend | React 19 + TypeScript frontend | SQLite + SQLx database | uPlot charts

---

## 📝 Release Notes Template

All future releases will be documented using the following format:

```markdown
## vX.Y.Z - [Title/Name] 📅 [Release Date]

### 📋 Overview
Brief description of the main purpose, goals, and what this release brings to users (2-3 sentences).

### ✨ New Features
- Feature 1: Description
- Feature 2: Description
- Feature 3: Description

### 🔧 Improvements
- Improvement 1: Description
- Improvement 2: Description
- Improvement 3: Description

### 🐛 Bug Fixes
- Bug 1: Description
- Bug 2: Description

### ⚡ Performance Optimizations
- Optimization 1: Description
- Optimization 2: Description

### 🚀 Breaking Changes
List of changes that are incompatible with previous versions, if any.

### 📦 Technical Details
- Library updates
- Database schema changes
- Architectural changes (if applicable)

### 🗑️ Deprecated Features
Features that will be removed in future versions, if any.

### 📥 Installation & Upgrade
Installation instructions and upgrade steps from previous versions.

### 🐛 Known Issues
Known issues in this release and their workarounds.

### 🙏 Contributors
Names of contributors to this release.

### 📚 Documentation & Resources
Relevant documentation links.

### 📝 Notes
Additional important information, if any.
```

### 📋 Template Usage Guide

1. **Title:** Use version number (semantic versioning), short name, and emoji
2. **Overview:** 2-3 sentences summarizing what this release brings
3. **New Features:** List all new features as bullet points
4. **Improvements:** Document performance, UX, and other improvements
5. **Bug Fixes:** List resolved bugs (include issue numbers if available)
6. **Breaking Changes:** Always highlight - this is critical!
7. **Known Issues:** List current known issues and their workarounds
8. **Date:** Use ISO 8601 format (YYYY-MM-DD) or written format

### 📊 Versioning Scheme

DriveAnalizer uses **Semantic Versioning** (SemVer): `MAJOR.MINOR.PATCH`

- **MAJOR:** Major new features or breaking changes (0 → 1.0.0)
- **MINOR:** New features, backward compatible (0.1.0 → 0.2.0)
- **PATCH:** Bug fixes (0.1.0 → 0.1.1)

**Example Release Flow:**
- v0.1.0 - Initial release
- v0.1.1 - Bug fix
- v0.2.0 - New feature added
- v1.0.0 - Stable and production-ready version
