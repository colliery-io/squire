# Install the app

Squire's phone app is a single Android app. The same app becomes a child's player (**the Squire**)
or a parent's quick-admin (**the Knight**) depending on how it's [paired](pair-phone.md).

> **iPhone?** The phone app is Android-only today. Parents can still do everything from the
> [Keep](../reference/glossary.md#the-keep) in a browser.

## Get the APK

The app isn't on the Play Store — your **home server distributes it** over your LAN, always matching
the server version.

1. Make sure the [home server is running](../tutorials/self-host-setup.md) and the phone is on the
   same Wi‑Fi.
2. In the **Keep**, open the **Pair** tab. It shows a QR code that links to the app download on your
   LAN.

   ![The Keep — Pair tab](../images/keep-pair.png)

3. On the phone, scan that QR (or browse to the URL it encodes) and download the APK.

## Install it

1. Tap the downloaded `squire-<n>.apk`.
2. Android will ask permission to install apps from this source the first time — allow it for your
   browser/files app, then continue.
3. Open Squire once it installs. It will ask to be paired — keep that screen up and continue to
   [Pair a phone](pair-phone.md).

## Updates

You don't reinstall to update. The server hands out new versions over the LAN and the app updates
itself in place — see [Delivery & updates](../explanation/delivery-and-updates.md).
