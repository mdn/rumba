# Changelog

## [1.9.0](https://github.com/mdn/rumba/compare/v1.8.0...v1.9.0) (2024-04-04)


### Features

* **ai-help:** delete ai history after six months ([#437](https://github.com/mdn/rumba/issues/437)) ([92541a5](https://github.com/mdn/rumba/commit/92541a5af026d31868a3009ab2b35b618b2592cd))
* **ai-help:** log message metadata ([#424](https://github.com/mdn/rumba/issues/424)) ([d035afa](https://github.com/mdn/rumba/commit/d035afa782eb5934be5702c3c8ca173c503cf62e))


### Bug Fixes

* **ai-help:** add qa questions to trigger error ([#450](https://github.com/mdn/rumba/issues/450)) ([7572e5b](https://github.com/mdn/rumba/commit/7572e5b79b88a5f3a454bd9f542266b3957a71fd))
* **ai-help:** artificial error triggers properly in the chat phase ([#452](https://github.com/mdn/rumba/issues/452)) ([deb9d54](https://github.com/mdn/rumba/commit/deb9d5443d4c9e338a6e16fdd94bb6f8bfecebda))
* **ai-help:** avoid spawning thread for history if history is disabled ([#438](https://github.com/mdn/rumba/issues/438)) ([706ce23](https://github.com/mdn/rumba/commit/706ce239d0deb4bbb8acb400806b2e7052c5da74))
* **ai-help:** configurable artificial error triggers ([#445](https://github.com/mdn/rumba/issues/445)) ([9c53f66](https://github.com/mdn/rumba/commit/9c53f66823d75b224d73212b8aa4b61dfa1d7945))
* **newsletter:** validate email ([#454](https://github.com/mdn/rumba/issues/454)) ([cd7c0f9](https://github.com/mdn/rumba/commit/cd7c0f95460a061baa7688559aa89f73baf42ee4))
* **updates:** continue on release without date, not break ([#447](https://github.com/mdn/rumba/issues/447)) ([c0949ca](https://github.com/mdn/rumba/commit/c0949ca083be84d5f27bcee8feb8966162250e59))


### Enhancements

* **ai-help:** record embedding duration and model separately ([#458](https://github.com/mdn/rumba/issues/458)) ([20760b2](https://github.com/mdn/rumba/commit/20760b2676f85deb04797f29a1e075d9b42f4ccf))


### Miscellaneous

* **ai-help:** stop generating off-topic answer ([#455](https://github.com/mdn/rumba/issues/455)) ([7dc8a6e](https://github.com/mdn/rumba/commit/7dc8a6e906517732b5b1d07332d21bb2dd67df0b))

## [1.8.0](https://github.com/mdn/rumba/compare/v1.7.0...v1.8.0) (2024-03-04)


### Features

* **ai-help:** add parent short_title for duplicate source titles ([#428](https://github.com/mdn/rumba/issues/428)) ([f523d52](https://github.com/mdn/rumba/commit/f523d5273390f2465b396db1a969bb5baa18e78b))
* **ai-help:** bump GPT-3.5 Turbo model ([#429](https://github.com/mdn/rumba/issues/429)) ([688e35d](https://github.com/mdn/rumba/commit/688e35d8669ea3caa011c3847fb501fc338223c2))
* **ai-test:** add --no-subscription flag ([#413](https://github.com/mdn/rumba/issues/413)) ([e6874e6](https://github.com/mdn/rumba/commit/e6874e602d488df67d74c44591b42b0e5b486f8c))
* **plus:** user subscription transitions ([#415](https://github.com/mdn/rumba/issues/415)) ([604b0e7](https://github.com/mdn/rumba/commit/604b0e7e4672e49a3501987aa5faef6ef07686a2))
* **plus:** user subscription transitions ([#421](https://github.com/mdn/rumba/issues/421)) ([5a307ca](https://github.com/mdn/rumba/commit/5a307cac964a693657f40b9d59f02fc837ed4b88))


### Bug Fixes

* **ai-help:** reset user quota on openai api error ([#430](https://github.com/mdn/rumba/issues/430)) ([cd6fdf8](https://github.com/mdn/rumba/commit/cd6fdf8e34b7bc17379e03efa922b99b54f8091d))
* **ai:** history fix ([#423](https://github.com/mdn/rumba/issues/423)) ([83f95a5](https://github.com/mdn/rumba/commit/83f95a51dd4f799777291b88044f268a5d5c24f7))
* **plus:** user subscription transitions ([#417](https://github.com/mdn/rumba/issues/417)) ([47b5a0b](https://github.com/mdn/rumba/commit/47b5a0b1d3d3bb8ed3267ccc4e9546c090fab33a))
* **tests:** add 10ms delay after we are done with stubr ([#408](https://github.com/mdn/rumba/issues/408)) ([18d8fda](https://github.com/mdn/rumba/commit/18d8fda52e2c419b9aeda7b3546704208d761f1f))


### Enhancements

* **ai-help:** format answers to off-topic questions ([#427](https://github.com/mdn/rumba/issues/427)) ([a545d66](https://github.com/mdn/rumba/commit/a545d66e1d91c9d0d76d89f85e459cc1fa34f5eb))


### Miscellaneous

* **deps:** update minor depedency versions ([#418](https://github.com/mdn/rumba/issues/418)) ([8994fc5](https://github.com/mdn/rumba/commit/8994fc5d5ed88ef425da0e1b9e9fb73486e7e886))
* **deps:** update non-breaking major deps ([#419](https://github.com/mdn/rumba/issues/419)) ([cda0c26](https://github.com/mdn/rumba/commit/cda0c26a7d79a9b0f13b628f296c4a907d4f1605))
* **workflows:** cache build artifacts ([#420](https://github.com/mdn/rumba/issues/420)) ([d32d6cf](https://github.com/mdn/rumba/commit/d32d6cf9f5d8349e650eef344a2494a3f3c9786e))

## [1.7.0](https://github.com/mdn/rumba/compare/v1.6.1...v1.7.0) (2024-01-31)


### Features

* **ai-help:** bump GPT-4 Turbo model ([#411](https://github.com/mdn/rumba/issues/411)) ([9c9038a](https://github.com/mdn/rumba/commit/9c9038ab258ed5bae635885c56567b425757c619))
* **ai-help:** switch to markdown context ([#410](https://github.com/mdn/rumba/issues/410)) ([6d71177](https://github.com/mdn/rumba/commit/6d711779d2c635fe18965422838ae79417d07650))


### Miscellaneous

* **workflows:** enable RUST_BACKTRACE for tests ([#390](https://github.com/mdn/rumba/issues/390)) ([3e7ab07](https://github.com/mdn/rumba/commit/3e7ab0704d12fa99905ec872960faf7bb89f4dcc))

## [1.6.1](https://github.com/mdn/rumba/compare/v1.6.0...v1.6.1) (2023-12-19)


### Bug Fixes

* **ai-help:** use GPT-3.5 for free users ([#393](https://github.com/mdn/rumba/issues/393)) ([94262d8](https://github.com/mdn/rumba/commit/94262d845e124b8f5176b314920d7aa81ce57f87))

## [1.6.0](https://github.com/mdn/rumba/compare/v1.5.1...v1.6.0) (2023-12-14)


### Features

* **ai-help:** release 2.0 ([#373](https://github.com/mdn/rumba/issues/373)) ([9499ee9](https://github.com/mdn/rumba/commit/9499ee9a183bed6bf7389bd83494d1f065f916d2))


### Miscellaneous

* **github:** add CODEOWNERS ([#385](https://github.com/mdn/rumba/issues/385)) ([e89284e](https://github.com/mdn/rumba/commit/e89284ed949378503cf4c3af498049ba36e9b62c))

## [1.5.1](https://github.com/mdn/rumba/compare/v1.5.0...v1.5.1) (2023-08-15)


### Bug Fixes

* **errors:** Downgrade AI/Playground erors to 400 ([#304](https://github.com/mdn/rumba/issues/304)) ([855094f](https://github.com/mdn/rumba/commit/855094fde7d2dd2793d1f6060aad89dafc83e793))

## [1.5.0](https://github.com/mdn/rumba/compare/v1.4.2...v1.5.0) (2023-07-24)


### Features

* **info:** add an info endpoint ([#301](https://github.com/mdn/rumba/issues/301)) ([9614323](https://github.com/mdn/rumba/commit/9614323ad0087898775400f5bbb081436b8614d9))

## [1.4.2](https://github.com/mdn/rumba/compare/v1.4.1...v1.4.2) (2023-07-07)


### Enhancements

* **ai-help:** Don't answer if no refs ([#277](https://github.com/mdn/rumba/issues/277)) ([5f9bb64](https://github.com/mdn/rumba/commit/5f9bb647928659a775a8b632f08f04ddbd45a6fe))
* **release-please:** take enhance/chore commits into consideration ([#282](https://github.com/mdn/rumba/issues/282)) ([f3dd4b1](https://github.com/mdn/rumba/commit/f3dd4b14028695598ed8c4c98d2791994fb1afad))

## [1.4.1](https://github.com/mdn/rumba/compare/v1.4.0...v1.4.1) (2023-07-05)


### Bug Fixes

* **play:** don't panic on to short id ([#273](https://github.com/mdn/rumba/issues/273)) ([46015de](https://github.com/mdn/rumba/commit/46015de19d30f87b9e5ea3287f0c474243eaf1c5))

## [1.4.0](https://github.com/mdn/rumba/compare/v1.3.1...v1.4.0) (2023-06-28)


### Features

* **ai-explain:** add ai-explain api ([#262](https://github.com/mdn/rumba/issues/262)) ([9785eab](https://github.com/mdn/rumba/commit/9785eab520301f275e6489fda10d1cd77c40df51))

## [1.3.1](https://github.com/mdn/rumba/compare/v1.3.0...v1.3.1) (2023-06-27)


### Bug Fixes

* **ai-help:** add related docs (negate missing) ([#257](https://github.com/mdn/rumba/issues/257)) ([634bf40](https://github.com/mdn/rumba/commit/634bf40d27d9a9f066e9cc1dc9378e020fb6f2d0))

## [1.3.0](https://github.com/mdn/rumba/compare/v1.2.0...v1.3.0) (2023-06-27)


### Features

* **plus:** add AI Help backend ([#230](https://github.com/mdn/rumba/issues/230)) ([064dedd](https://github.com/mdn/rumba/commit/064deddaa5ebec95d2a53a4c8b46fab276db4c34))

## [1.2.0](https://github.com/mdn/rumba/compare/v1.1.0...v1.2.0) (2023-06-19)


### Features

* **playground:** playground back-end ([#222](https://github.com/mdn/rumba/issues/222)) ([04a67ea](https://github.com/mdn/rumba/commit/04a67ea8452ec7b19752ea94de7aa60acb5b4a54))

## [1.1.0](https://github.com/mdn/rumba/compare/v1.0.0...v1.1.0) (2023-05-16)


### Features

* **newsletter:** support public double opt-in ([#187](https://github.com/mdn/rumba/issues/187)) ([e83d4ad](https://github.com/mdn/rumba/commit/e83d4adf54a77c800f3a438796a5974e55cc3f95))


### Bug Fixes

* **clippy:** fix derivable_impls ([#188](https://github.com/mdn/rumba/issues/188)) ([4860b43](https://github.com/mdn/rumba/commit/4860b43556104a584df8775ab53821301c2a4087))
