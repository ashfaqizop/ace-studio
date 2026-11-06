# Porting notes: Plasmo Voice → Ace Plasmo

Upstream: [plasmoapp/plasmo-voice](https://github.com/plasmoapp/plasmo-voice) `main` @ `b93ea39` (2.2.0-beta.2),
LGPL-3.0-only. Only the server side is ported; players keep the official client mod.

## Protocol

| Upstream | Here |
|---|---|
| `PacketUtil`, Guava `ByteArrayDataOutput` | `proto.rs`: big-endian numbers, modified UTF-8 `writeUTF` |
| `PacketTcpCodec` (ids 1-26 on `plasmo:voice/v2`) | `packets.rs`: the ids the server sends or reads |
| `PacketUdpCodec` (magic `0x4e9004e9`, type, secret, timestamp) | `packets.rs`: `decode_udp`, `udp_*` |
| `VoiceActivation.generateId` / `VoiceSourceLine.generateId` | precomputed `nameUUIDFromBytes` constants, checked in tests |

## Flow

1. Join → `PlayerInfoRequestPacket`, retried after 1, 3, 5, 10 and 15 s (`PlayerInfoRequestScheduler`).
2. `PlayerInfoPacket` (mod version + X.509 RSA public key) → version check → `ConnectionPacket` (secret, ip, port).
3. First UDP packet carrying that secret → connection: `ConfigPacket` (capture info, RSA-encrypted AES key, the
   proximity source line and activation, permissions, icon settings), `PlayerListPacket`, and a
   `PlayerInfoUpdatePacket` to everyone.
4. `PlayerAudioPacket` → `SourceAudioPacket` to connected players in the same world within
   `min(distance + max_extra_audio_broadcast_distance, 2 × distance)`, plus `SelfAudioInfoPacket` to the speaker.
   `SourceInfoPacket` goes out the first time a source talks (and after state changes); clients that missed it ask
   with `SourceInfoRequestPacket`.
5. `PlayerAudioEndPacket` → `SourceAudioEndPacket` on the plugin channel.
6. Keep-alive pings every 2.5-4 s; no UDP for `keep_alive_timeout_ms` → disconnect, then the handshake restarts.

The server never decodes or re-encodes audio. Clients encrypt Opus frames with the shared AES-CBC key, and the bytes
are forwarded untouched, exactly as upstream does.

## Pumpkin differences

- **Tick-paced UDP.** Pumpkin plugins run on the server tick, so the socket is drained every 50 ms instead of
  by Netty's event loop. Audio frames are 20 ms, so a listener can get two or three per tick. The client's jitter
  buffer smooths that out, but expect up to one extra tick of latency compared to Paper.
- **Positions** are read once per tick for connected players, outside the state lock (host calls must never run
  while the lock is held: re-entrancy is fatal in WASM).
- **Permissions** use the plugin namespace (`ace-plasmo:…` instead of `pv.…`), as Pumpkin requires.
- **Vanish** (`canSee`) isn't in the plugin API, so everyone with the mod is visible in the voice player list.
- **Languages**: only the strings the client needs from the server (`pv.activation.proximity`) are sent; server
  messages are English.

## Not ported yet

- Mutes (`VoiceMuteManager`, `/vmute`, `/vunmute`, `/vmutelist`) and `/vreload`.
- Mod-required kicks (`client_mod_required`).
- The service channel and forwarding secret (proxy setups).
- Plasmo add-ons (groups, broadcast, …): each is its own Java plugin and would be its own port.
