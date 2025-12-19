# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/florianduros/miou/compare/v0.1.1...v0.1.2)

### Added


- Display miou version at startup #40 - ([9f61d50](https://github.com/florianduros/miou/commit/9f61d502887f4ca3436028043d09b07bd30b201c))
- Add miou version to help command #39 - ([7a008dc](https://github.com/florianduros/miou/commit/7a008dc4f53934caebceb7994c84fd0822abdd19))

### Fixed


- Stop tmars api polling when 501 or 503 error is hit #33 - ([5697f69](https://github.com/florianduros/miou/commit/5697f69591c85b0e2016460272413abd918e8a5e))
- Retry sync after 2mins #45 - ([d21dfcc](https://github.com/florianduros/miou/commit/d21dfcc98bc6aa0679bb60cac53e6afa6f3aa69f))
- Avoid panicking when there is no avatar - ([6e769a6](https://github.com/florianduros/miou/commit/6e769a60a4bc22917f697a2c35773e49f58faf82))

### Other


- Catch bot startup error - ([0a83631](https://github.com/florianduros/miou/commit/0a836316c23fa44911dd41f37e5235de5f174f12))
- Improve changelog generation - ([c1be31e](https://github.com/florianduros/miou/commit/c1be31e5ba3405bd542b2f5e090fd15c0787fb59))


## [0.1.1](https://github.com/florianduros/miou/compare/v0.1.0...v0.1.1) - 2025-12-15

### Fixed

- remove trailing slash in url
- add user_id to alert message

### Other

- use &str instead of string for function parameter
- remove docrs readme
- remove release artifacts

## [0.1.0](https://github.com/florianduros/miou/releases/tag/v0.1.0) - 2025-12-15

### Added

- first implementation
