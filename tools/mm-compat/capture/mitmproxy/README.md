# mitmproxy Capture Guide for Mattermost Mobile

Follow these steps to capture API traffic from the Mattermost Mobile app.

## 1. Setup mitmproxy
Run the start script to begin the proxy:
```bash
./tools/mm-compat/capture/start_proxy.sh
```
This will start mitmproxy on port 8080 and save all flows to `flows.mitm`.

## 2. Configure Android Emulator
1. Start your Android Emulator.
2. Go to **Settings > Network & Internet > Internet**.
3. Tap the **Gear icon** next to AndroidWifi.
4. Tap the **Pencil icon** (edit) at the top right.
5. Expand **Advanced options**.
6. Set **Proxy** to `Manual`.
7. Set **Proxy hostname** to your machine's IP (not localhost/127.0.0.1).
8. Set **Proxy port** to `8080`.
9. Save.

## 3. Install CA Certificate
1. Open Chrome on the emulator and go to `http://mitm.it`.
2. Download the Android certificate.
3. Go to **Settings > Security > More security settings > Encryption & credentials > Install a certificate > CA certificate**.
4. Install anyway and select the downloaded `mitmproxy-ca-cert.cer`.

## 4. Record Journeys
Perform the following in the app:
1. **Server Setup**: Add your RustChat server URL.
2. **Login**: Authenticate with a test user.
3. **Sync**: Let the app load channels and posts.
4. **Interaction**: Send messages, open threads, react to posts.
5. **Foreground/Background**: Minimize and restore the app.

## 5. Export Traffic
Once done, stop mitmproxy (Ctrl+C). The script will automatically run `export_flows.py` or you can run it manually:
```bash
python3 tools/mm-compat/capture/export_flows.py
```
This produces `tools/mm-compat/output/endpoints_capture.json`.
