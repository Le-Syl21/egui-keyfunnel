# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-07-07

Initial release.

### Added
- **`KeyFunnel`** — an `egui::Plugin` that funnels keyboard events (`Event::Key`,
  `Event::Text`, `Event::Ime`) from secondary viewports to a single target
  viewport (the root by default), so keyboard input reaches the main UI even when
  a compositor focuses a secondary window. Pointer, touch and scroll are left
  untouched.
- Builder methods `to_viewport` (choose the target) and `from_viewports`
  (restrict the sources).
- Two-viewport `funnel_demo` example.
- Targets egui 0.35.
