# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/florianduros/miou/compare/v0.1.2...v0.1.3)

### Fixed


- Format correctly user id in mention - ([4282375](https://github.com/florianduros/miou/commit/4282375548b6d6545d3a499973f1d663505229f8))
- Keep game and alerts on error from tmars server - ([d4d807b](https://github.com/florianduros/miou/commit/d4d807b2f0b3264266a38a282f79cec1643600c1))

### Other


- *(deps)* Bump reqwest in the dependencies group - ([81a5e77](https://github.com/florianduros/miou/commit/81a5e772e2d5e36fabf02a127bc7957123d57443))
- *(deps)* Bump openssl from 0.10.75 to 0.10.78 - ([f3a2dbe](https://github.com/florianduros/miou/commit/f3a2dbee174b26c28744a8f6ffe568f43d526766))
- *(deps)* Bump the dependencies group with 2 updates - ([1bbd8eb](https://github.com/florianduros/miou/commit/1bbd8ebb19e50ef77ad1308dc0a02ed15bb8886c))
- *(deps)* Bump the dependencies group with 3 updates - ([aacf317](https://github.com/florianduros/miou/commit/aacf3172d91e1e27f8fecc131340914f409caa50))
- *(deps)* Bump the dependencies group across 1 directory with 10 updates - ([7f58d22](https://github.com/florianduros/miou/commit/7f58d223112b2d1afe6bdfce0f6bb0e234efa84b))
- Merge pull request #73 from florianduros/dependabot/cargo/quinn-proto-0.11.14 - ([7967ccf](https://github.com/florianduros/miou/commit/7967ccf431646106f7659434ffc2adbbf903e58d))
- Merge pull request #86 from florianduros/dependabot/cargo/rustls-webpki-0.103.13 - ([3d1eaff](https://github.com/florianduros/miou/commit/3d1eaff47d0f8d32daf55c59f0a8bb581cdf4987))
- Merge pull request #88 from florianduros/dependabot/cargo/openssl-0.10.78 - ([e90a65c](https://github.com/florianduros/miou/commit/e90a65c91eabf89569561ec9504b9cbbb22cea96))
- Use rust toolchain 1.93.1 - ([8f927e4](https://github.com/florianduros/miou/commit/8f927e41c94424497435f32bfc0fd1d54af0fb54))
- Use toyko rw instead of mutex - ([c2dec4d](https://github.com/florianduros/miou/commit/c2dec4df5d817cc6833a6a8e3ea4f3bc8bfda471))
- Merge branch 'main' into dependabot/cargo/dependencies-e1dd654884 - ([84be002](https://github.com/florianduros/miou/commit/84be002ccaee25420ab60c832a2895c997ca6d25))
- Move mockito to dev deps - ([3d8b13e](https://github.com/florianduros/miou/commit/3d8b13ea350b79dc87ac9ddeade388c955e84431))


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
