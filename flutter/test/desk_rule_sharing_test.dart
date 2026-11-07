import 'package:flutter_hbb/generated_bridge.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_hbb/models/server_model.dart';
import 'package:flutter_test/flutter_test.dart';

class _Binding extends Fake implements RustdeskImpl {
  @override
  String translate(
          {required String name, required String locale, dynamic hint}) =>
      name;
}

class _Session extends Fake implements FFI {}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('Android sharing events update the visible sharing state', () {
    platformFFI.setBindingForTesting(_Binding());
    final session = _Session();
    final model = ServerModel(WeakReference(session));

    model.changeStatue('sharing', true);
    expect(model.isStart, isTrue);
    model.changeStatue('sharing', false);
    expect(model.isStart, isFalse);
  });
}
