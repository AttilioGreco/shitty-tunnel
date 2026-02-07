# Docker Test Keys

⚠️ **SOLO PER TEST - Non usare in produzione!**

Queste chiavi sono committate nel repository solo per facilitare i test locali.

## Server Keys

```toml
private_key = "qWKxPYzW1r1O1heyXqRJi0vD4YyCEO1E2s4fx6+Lndo="
public_key  = "fnTd+PXU/63ZC3wOBJDAlZiwAW6ChM1aDpMH5hFPAgw="
```

## Client Keys

```toml
private_key = "xxSnB3yx/kbIBIUwarSfo+Kq+EjiEr/uvedzxLsVdbw="
public_key  = "rNjPolajKAEIA79yYIc2HtYeCUHeJ5eHHxREoKzLffk="
```

## Corrispondenza Chiavi

✅ **IMPORTANTE:** Le chiavi devono corrispondere così:

### server.toml
```toml
[server]
private_key = "qWKxPYzW1r1O1heyXqRJi0vD4YyCEO1E2s4fx6+Lndo="  # Server private

[[peers]]
public_key = "rNjPolajKAEIA79yYIc2HtYeCUHeJ5eHHxREoKzLffk="   # Client public ← DEVE CORRISPONDERE
domain = "test.localhost"
```

### client.toml
```toml
[client]
private_key = "xxSnB3yx/kbIBIUwarSfo+Kq+EjiEr/uvedzxLsVdbw="       # Client private
server_public_key = "fnTd+PXU/63ZC3wOBJDAlZiwAW6ChM1aDpMH5hFPAgw="  # Server public ← DEVE CORRISPONDERE
```

## Verifica

La chiave pubblica deriva crittograficamente dalla chiave privata:

- Server private → genera automaticamente → Server public
- Client private → genera automaticamente → Client public

Quando il client si connette:
1. Server verifica che Client public (nei peers) corrisponda alla firma
2. Client verifica che Server public corrisponda alla firma ricevuta

## Come rigenerare

```bash
# Genera coppia server
cargo run --bin shitty-tunnel -- keygen

# Genera coppia client
cargo run --bin shitty-tunnel -- keygen

# Aggiorna:
# - server.toml: server private + client public
# - client.toml: client private + server public
```

## Note

- Le chiavi Ed25519 sono codificate in **base64** (44 caratteri)
- **Mutual authentication**: sia server che client si verificano a vicenda
- **Domain matching**: Il server instrada le richieste HTTP basandosi sull'header `Host`
- Per produzione: genera chiavi nuove e mantienile segrete!
