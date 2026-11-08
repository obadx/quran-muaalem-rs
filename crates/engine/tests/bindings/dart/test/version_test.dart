// Smoke test for the BoltFFI Dart binding. Run via tests/bindings/run.sh dart.
import 'dart:io';

import 'package:quran_muaalem_engine/quran_muaalem_engine.dart' as engine;
import 'package:test/test.dart';

void main() {
  test('version() matches the crate version', () {
    final version = engine.version();
    expect(version, matches(RegExp(r'^\d+\.\d+\.\d+')));

    final expected = Platform.environment['ENGINE_VERSION'];
    if (expected != null && expected.isNotEmpty) {
      expect(version, equals(expected));
    }
  });
}
