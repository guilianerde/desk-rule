import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:window_manager/window_manager.dart';

import '../../common.dart';
import '../../consts.dart';
import '../../models/platform_model.dart';

class DeskRuleHome extends StatefulWidget {
  const DeskRuleHome({super.key});

  @override
  State<DeskRuleHome> createState() => _DeskRuleHomeState();
}

class _DeskRuleHomeState extends State<DeskRuleHome> {
  final _address = TextEditingController();
  final _fingerprint = TextEditingController();
  final _form = GlobalKey<FormState>();
  Timer? _timer;
  String _hostKey = '';
  String _passcode = '';
  bool _sharing = false;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    gFFI.serverModel.addListener(_onServerChanged);
    unawaited(_refreshHost());
    _timer = Timer.periodic(
        const Duration(seconds: 2), (_) => unawaited(_refreshHost()));
  }

  void _onServerChanged() => unawaited(_refreshHost());

  Future<void> _refreshHost() async {
    if (_loading || isIOS) return;
    _loading = true;
    try {
      final sharing = isAndroid
          ? gFFI.serverModel.isStart
          : await bind.mainGetOption(key: kOptionStopService) == 'N';
      final key = sharing ? await bind.mainGetFingerprint() : '';
      final password = sharing ? await bind.mainGetTemporaryPassword() : '';
      if (!mounted) return;
      setState(() {
        _sharing = sharing;
        _hostKey = key;
        _passcode = password;
      });
    } finally {
      _loading = false;
    }
  }

  Future<void> _toggleHost() async {
    if (isAndroid) {
      await gFFI.serverModel.toggleService();
    } else if (_sharing) {
      await bind.mainStopService();
    } else {
      if (isMacOS) {
        final screen = bind.mainIsCanScreenRecording(prompt: true);
        final input = bind.mainIsProcessTrusted(prompt: true);
        if (!screen || !input) return;
      }
      await bind.mainStartService();
    }
    await _refreshHost();
  }

  bool _isIpAddress(String value) {
    if (InternetAddress.tryParse(value) != null) return true;
    final endpoint =
        RegExp(r'^(?:\[([^\]]+)\]|([^:]+)):(\d+)$').firstMatch(value);
    if (endpoint == null) return false;
    final port = int.tryParse(endpoint.group(3)!);
    return port != null &&
        port > 0 &&
        port <= 65535 &&
        InternetAddress.tryParse(endpoint.group(1) ?? endpoint.group(2)!) !=
            null;
  }

  Future<void> _connect() async {
    if (!_form.currentState!.validate()) return;
    final address = _address.text.trim();
    await bind.mainSetLocalOption(
        key: 'peer-fingerprint-$address',
        value: _fingerprint.text.replaceAll(RegExp(r'\s'), ''));
    if (mounted) await connect(context, address);
  }

  Future<void> _setPermanentPassword() async {
    final password = TextEditingController();
    final confirmation = TextEditingController();
    final value = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Set permanent password'),
        content: Column(mainAxisSize: MainAxisSize.min, children: [
          TextField(controller: password, obscureText: true,
              decoration: const InputDecoration(labelText: 'Password')),
          TextField(controller: confirmation, obscureText: true,
              decoration: const InputDecoration(labelText: 'Confirm password')),
        ]),
        actions: [
          TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
          TextButton(onPressed: () {
            if (password.text.isNotEmpty && password.text == confirmation.text) {
              Navigator.pop(context, password.text);
            }
          }, child: const Text('Save')),
        ],
      ),
    );
    password.dispose();
    confirmation.dispose();
    if (value == null) return;
    final saved = await bind.mainSetPermanentPasswordWithResult(password: value);
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(
        content: Text(saved ? 'Permanent password saved' : 'Could not save password'),
      ));
    }
  }

  Widget _copyable(String label, String value) {
    return ListTile(
      contentPadding: EdgeInsets.zero,
      title: Text(label),
      subtitle: SelectableText(value.isEmpty ? '…' : value),
      trailing: IconButton(
        tooltip: 'Copy $label',
        onPressed: value.isEmpty
            ? null
            : () => Clipboard.setData(ClipboardData(text: value)),
        icon: const Icon(Icons.copy),
      ),
    );
  }

  @override
  void dispose() {
    _timer?.cancel();
    gFFI.serverModel.removeListener(_onServerChanged);
    _address.dispose();
    _fingerprint.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: isDesktop
          ? null
          : AppBar(title: const Text('Desk Rule')),
      body: SafeArea(
        child: Center(
          child: SizedBox(
            width: 480,
            child: ListView(
              shrinkWrap: true,
              padding: const EdgeInsets.all(24),
              children: [
                Text('Control a device',
                    style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 12),
                Form(
                  key: _form,
                  child: Column(
                    children: [
                      TextFormField(
                        controller: _address,
                        decoration: const InputDecoration(
                          labelText: 'Device IP address',
                          hintText: '100.64.0.10',
                        ),
                        autocorrect: false,
                        onChanged: (value) {
                          _fingerprint.text = bind.mainGetLocalOption(
                              key: 'peer-fingerprint-${value.trim()}');
                        },
                        validator: (value) => _isIpAddress(value!.trim())
                            ? null
                            : 'Enter an IPv4 or IPv6 address',
                      ),
                      TextFormField(
                        controller: _fingerprint,
                        decoration:
                            const InputDecoration(labelText: 'Device fingerprint'),
                        autocorrect: false,
                        validator: (value) => RegExp(r'^[0-9a-fA-F]{64}$')
                                .hasMatch(value!.replaceAll(RegExp(r'\s'), ''))
                            ? null
                            : 'Paste the fingerprint shown on the controlled device',
                        onFieldSubmitted: (_) => _connect(),
                      ),
                      const SizedBox(height: 12),
                      Align(
                        alignment: Alignment.centerRight,
                        child: FilledButton(
                            onPressed: _connect, child: const Text('Connect')),
                      ),
                    ],
                  ),
                ),
                if (!isIOS) ...[
                  const Divider(height: 32),
                  SwitchListTile(
                    contentPadding: EdgeInsets.zero,
                    title: const Text('Allow remote control'),
                    subtitle: isAndroid && _sharing && !gFFI.serverModel.mediaOk
                        ? const Text('Allow screen sharing in the Android prompt')
                        : null,
                    value: _sharing,
                    onChanged: (_) => _toggleHost(),
                  ),
                  if (isAndroid)
                    SwitchListTile(
                      contentPadding: EdgeInsets.zero,
                      title: const Text('Allow touch and keyboard control'),
                      subtitle: const Text('Enable Desk Rule in Android accessibility settings'),
                      value: gFFI.serverModel.inputOk,
                      onChanged: (_) => gFFI.serverModel.toggleInput(),
                    ),
                  if (_sharing) ...[
                    TextButton(onPressed: _setPermanentPassword,
                        child: const Text('Set permanent password')),
                    const Text('Share this device’s IP address, fingerprint and temporary password with your controller.'),
                    _copyable('Fingerprint', _hostKey),
                    _copyable('Temporary password', _passcode),
                  ],
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}
