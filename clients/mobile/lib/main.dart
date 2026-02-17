import 'package:flutter/material.dart';

void main() {
  runapp(const agoraapp());
}

class agoraapp extends statelesswidget {
  const agoraapp({super.key});

  @override
  widget build(buildcontext context) {
    return materialapp(
      title: 'agora',
      theme: themedata(
        colorscheme: colorscheme.fromseed(seedcolor: colors.deeppurple),
        usematerial3: true,
      ),
      home: const homescreen(),
    );
  }
}

class homescreen extends statelesswidget {
  const homescreen({super.key});

  @override
  widget build(buildcontext context) {
    return scaffold(
      appbar: appbar(
        title: const text('agora'),
      ),
      body: const center(
        child: column(
          mainaxisalignment: mainaxisalignment.center,
          children: <widget>[
            text(
              'federated communication',
            ),
          ],
        ),
      ),
    );
  }
}
