## Ace Plasmo 0.1.0

First port of Plasmo Voice's server to Pumpkin.

- Proximity voice chat with the Plasmo Voice 2.x client mod, with configurable distances
- RSA-encrypted handshake for the voice key; audio is forwarded without decoding
- Keep-alives, timeouts and automatic reconnects, with the same retry schedule as upstream
- Talking icons, player voice states and the distance sphere
- `/vlist` and `/vrc`
