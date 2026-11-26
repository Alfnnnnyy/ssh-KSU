import {
  Adb,
  AdbDaemonTransport,
  AdbPacket,
  AdbPacketSerializeStream,
} from '@yume-chan/adb';
import {
  AdbScrcpyClient,
  AdbScrcpyOptions3_1,
} from '@yume-chan/adb-scrcpy';
import AdbWebCredentialStore from '@yume-chan/adb-credential-web';
import {
  AndroidKeyCode,
  AndroidKeyEventAction,
  AndroidMotionEventAction,
} from '@yume-chan/scrcpy';
import {
  BitmapVideoFrameRenderer,
  WebCodecsVideoDecoder,
  WebGLVideoFrameRenderer,
} from '@yume-chan/scrcpy-decoder-webcodecs';
import {
  Consumable,
  ReadableStream,
  StructDeserializeStream,
  pipeFrom,
} from '@yume-chan/stream-extra';

export interface RemoteSession {
  client: AdbScrcpyClient<AdbScrcpyOptions3_1<boolean>>;
  decoder: WebCodecsVideoDecoder;
  rendererElement: HTMLCanvasElement;
  disconnect: () => void;
  sendTouch: (action: 'down' | 'move' | 'up', x: number, y: number) => Promise<void>;
  sendKey: (key: 'back' | 'home' | 'power' | 'recents') => Promise<void>;
}

function createAdbConnectionFromWs(ws: WebSocket) {
  const rawReadable = new ReadableStream<Uint8Array>({
    start(controller) {
      ws.binaryType = 'arraybuffer';
      ws.onmessage = (event) => {
        if (event.data instanceof ArrayBuffer) {
          controller.enqueue(new Uint8Array(event.data));
        }
      };
      ws.onclose = () => {
        try {
          controller.close();
        } catch {
          // ignore
        }
      };
      ws.onerror = (e) => {
        try {
          controller.error(e);
        } catch {
          // ignore
        }
      };
    },
    cancel() {
      ws.close();
    },
  });

  const readable = rawReadable.pipeThrough(new StructDeserializeStream(AdbPacket));
  const writable = pipeFrom(
    new Consumable.WritableStream<Uint8Array>({
      write(chunk) {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(chunk);
        }
      },
    }),
    new AdbPacketSerializeStream()
  );
  return { readable, writable };
}

export async function connectRemoteScreen(
  targetAddress: string,
  onStatus: (msg: string) => void
): Promise<RemoteSession> {
  const target = targetAddress.includes(':') ? targetAddress : `${targetAddress}:5555`;
  onStatus(`Menghubungkan ke ${target}...`);

  const wsUrl = `ws://127.0.0.1:2246/adb?target=${encodeURIComponent(target)}`;
  const ws = new WebSocket(wsUrl);

  const { promise, resolve, reject } = Promise.withResolvers<void>();
  ws.onopen = () => resolve();
  ws.onerror = () => reject(new Error(`Gagal membuka koneksi ke ${target} (periksa apakah Wireless ADB menyala).`));
  await promise;
  onStatus('Koneksi terbuka. Mengautentikasi ADB...');
  const credentialStore = new AdbWebCredentialStore('RemoteSH');
  const connection = createAdbConnectionFromWs(ws);

  const transport = await AdbDaemonTransport.authenticate({
    serial: target,
    connection,
    credentialStore,
  });

  const adb = new Adb(transport);
  onStatus('ADB terhubung. Menyiapkan scrcpy-server...');

  // Fetch pre-bundled scrcpy-server binary
  const serverRes = await fetch('./scrcpy-server');
  if (!serverRes.ok) {
    throw new Error('File scrcpy-server tidak ditemukan di webroot.');
  }
  const serverBytes = await serverRes.arrayBuffer();

  const sync = await adb.sync();
  try {
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new Uint8Array(serverBytes));
        controller.close();
      },
    });
    await sync.write({
      filename: '/data/local/tmp/scrcpy-server.jar',
      file: stream,
    });
  } finally {
    await sync.dispose();
  }

  onStatus('Menjalankan scrcpy di perangkat target...');
  const options = new AdbScrcpyOptions3_1({
    tunnelForward: true,
    maxSize: 1280,
    videoBitRate: 4_000_000,
    maxFps: 60,
    audio: false,
  });

  const client = await AdbScrcpyClient.start(
    adb,
    '/data/local/tmp/scrcpy-server.jar',
    options
  );

  if (!client.videoStream) {
    throw new Error('Perangkat tidak mengirim video stream.');
  }

  const { metadata, stream: videoStream } = await client.videoStream;
  onStatus(`Menerima video (${metadata.codec})...`);

  let canvas: HTMLCanvasElement;
  let renderer: WebGLVideoFrameRenderer | BitmapVideoFrameRenderer;

  if (WebGLVideoFrameRenderer.isSupported) {
    const r = new WebGLVideoFrameRenderer();
    renderer = r;
    canvas = r.canvas as HTMLCanvasElement;
  } else {
    const r = new BitmapVideoFrameRenderer();
    renderer = r;
    canvas = r.canvas as HTMLCanvasElement;
  }

  const decoder = new WebCodecsVideoDecoder({
    codec: metadata.codec,
    renderer,
  });

  void videoStream.pipeTo(decoder.writable).catch(() => {});

  let screenWidth = 1080;
  let screenHeight = 2400;

  decoder.sizeChanged(({ width, height }) => {
    screenWidth = width;
    screenHeight = height;
    canvas.width = width;
    canvas.height = height;
  });

  onStatus('Layar terhubung!');

  const controller = client.controller;

  return {
    client,
    decoder,
    rendererElement: canvas,
    disconnect() {
      try {
        ws.close();
      } catch {}
    },
    async sendTouch(action, x, y) {
      if (!controller) return;
      const motionAction =
        action === 'down'
          ? AndroidMotionEventAction.Down
          : action === 'move'
            ? AndroidMotionEventAction.Move
            : AndroidMotionEventAction.Up;

      const px = Math.min(Math.max(0, Math.round(x)), screenWidth);
      const py = Math.min(Math.max(0, Math.round(y)), screenHeight);

      await controller.injectTouch({
        action: motionAction,
        pointerId: 0n,
        pointerX: px,
        pointerY: py,
        videoWidth: screenWidth,
        videoHeight: screenHeight,
        pressure: 1,
        actionButton: 0,
        buttons: action === 'up' ? 0 : 1,
      });
    },
    async sendKey(key) {
      if (!controller) return;
      if (key === 'back') {
        await controller.backOrScreenOn(AndroidKeyEventAction.Down);
        await controller.backOrScreenOn(AndroidKeyEventAction.Up);
      } else if (key === 'home') {
        await controller.injectKeyCode({
          action: 0,
          keyCode: AndroidKeyCode.AndroidHome,
          metaState: 0,
          repeat: 0,
        });
        await controller.injectKeyCode({
          action: 1,
          keyCode: AndroidKeyCode.AndroidHome,
          metaState: 0,
          repeat: 0,
        });
      } else if (key === 'power') {
        await controller.injectKeyCode({
          action: 0,
          keyCode: AndroidKeyCode.Power,
          metaState: 0,
          repeat: 0,
        });
        await controller.injectKeyCode({
          action: 1,
          keyCode: AndroidKeyCode.Power,
          metaState: 0,
          repeat: 0,
        });
      } else if (key === 'recents') {
        await controller.injectKeyCode({
          action: 0,
          keyCode: 187, // APP_SWITCH
          metaState: 0,
          repeat: 0,
        });
        await controller.injectKeyCode({
          action: 1,
          keyCode: 187,
          metaState: 0,
          repeat: 0,
        });
      }
    },
  };
}
