# Kubernetes Deployment

Simple Kubernetes deployment for shitty-tunnel using Gateway API.

## Files

- `server-deployment.yaml` - Deployment + ConfigMap
- `server-service.yaml` - Service (HTTP + gRPC)
- `gateway.yaml` - Gateway API configuration for HTTP and gRPC

> Note: these manifests do **not** set `metadata.namespace`, so apply commands in this document use `-n shitty-tunnel`.

## Prerequisites

- Kubernetes cluster v1.26+
- Gateway API CRDs installed (v1.0.0 or later)
- A Gateway API compatible controller (e.g., nginx-gateway-fabric, Istio, Envoy Gateway)

## Ports and protocols

| Component | Port | Protocol | Purpose |
|---|---:|---|---|
| Service `shitty-tunnel-server` | 8080 | HTTP | Public proxied traffic |
| Service `shitty-tunnel-server` | 50051 | gRPC (HTTP/2) | Tunnel client connections |
| Gateway listener `https` | 443 | HTTPS | Public wildcard hostname |
| Gateway listener `grpc` | 443 | HTTPS (gRPC) | Dedicated tunnel hostname |

## Setup

### 1. Create namespace

```bash
kubectl create namespace shitty-tunnel
```

### 2. Generate and configure keys

```bash
# Generate server keys
docker run --rm ghcr.io/attiliogreco/shitty-tunnel:latest keygen

# Generate client keys
docker run --rm ghcr.io/attiliogreco/shitty-tunnel:latest keygen
```

Edit `server-deployment.yaml`:
- In `ConfigMap.server.toml`, set `private_key` to the **server private key**.
- In `[[peers]]`, set:
  - `public_key` = **client public key**
  - `domain` = domain allowed for that client

Expected mapping:
- **Server private key**: used only by the server (`private_key`)
- **Client public key**: added to server allowlist (`[[peers]].public_key`)

For production, do not store private keys directly in a ConfigMap.
Prefer Kubernetes `Secret` + external secret management (e.g. Sealed Secrets, External Secrets, Vault).

### 3. Update domains

Edit `gateway.yaml`:
- Replace `*.your-domain.com` with your actual domain (e.g., `*.example.com`)
- Replace `tunnel.your-domain.com` with your tunnel endpoint (e.g., `tunnel.example.com`)
- Update `gatewayClassName` to match your Gateway controller (e.g., `nginx`, `istio`, `envoy`)
- If using cert-manager, uncomment the `cert-manager.io/cluster-issuer` annotation

### 4. Deploy

```bash
kubectl apply -n shitty-tunnel -f server-deployment.yaml
kubectl apply -n shitty-tunnel -f server-service.yaml
kubectl apply -n shitty-tunnel -f gateway.yaml
```

## Gateway API Configuration

Gateway API provides a more flexible and extensible way to configure traffic routing compared to Ingress.

### Gateway Resource
- Defines listeners for HTTP, HTTPS, and gRPC traffic
- Manages TLS termination
- Supports multiple protocols on different ports

### HTTPRoute (Public Proxy)
- **Hostname**: `*.your-domain.com` (wildcard for all subdomains)
- **Backend Port**: 8080
- **Protocol**: HTTP/HTTPS
- **Purpose**: Routes public HTTP traffic to backend services

### GRPCRoute (Tunnel Connections)
- **Hostname**: `tunnel.your-domain.com` (dedicated subdomain)
- **Backend Port**: 50051
- **Protocol**: gRPC over HTTPS (HTTP/2)
- **Purpose**: Routes client tunnel connections
- **Features**:
  - Session persistence for long-lived connections
  - gRPC service method matching
  - Automatic HTTP/2 support

## Client Configuration

Clients connect to the gRPC endpoint:

```toml
[client]
server_host = "https://tunnel.your-domain.com:443"
# or simply
server_host = "https://tunnel.your-domain.com"
```

## Verify

```bash
# Check pods
kubectl get pods -n shitty-tunnel

# Check Gateway and Routes
kubectl get gateway -n shitty-tunnel
kubectl get httproute -n shitty-tunnel
kubectl get grpcroute -n shitty-tunnel

# Check logs
kubectl logs -f deployment/shitty-tunnel-server -n shitty-tunnel
```

## Test

```bash
# Test HTTP (from browser or curl)
curl https://subdomain.your-domain.com

# Test gRPC (from client)
shitty-tunnel client --config client.toml
```

## Troubleshooting

### Pods not starting

```bash
kubectl describe pod -n shitty-tunnel <pod-name>
```

### gRPC connection fails

Check Gateway controller status:
```bash
kubectl get gateway shitty-tunnel-gateway -n shitty-tunnel -o yaml
kubectl get grpcroute shitty-tunnel-grpc-route -n shitty-tunnel -o yaml
```

Ensure your Gateway controller supports GRPCRoute. Check controller logs:
```bash
kubectl logs -n <controller-namespace> <controller-pod>
```

Also verify DNS and certificates:
- `tunnel.your-domain.com` must resolve to your Gateway address
- TLS cert for `tunnel.your-domain.com` must be valid and attached to the `grpc` listener

### Certificate issues

Check cert-manager is working:
```bash
kubectl get certificate -n shitty-tunnel
kubectl describe certificate shitty-tunnel-grpc-tls -n shitty-tunnel
```

## Notes

- Gateway API is the successor to Ingress and provides better support for advanced routing
- GRPCRoute requires a Gateway controller that supports the feature (check controller documentation)
- TLS/SSL is required for gRPC over Gateway API
- Gateway API CRDs must be installed before deploying the gateway resources
- Manifest names and image use `shitty-tunnel` (hyphenated naming)
- Recommended Gateway controllers:
  - nginx-gateway-fabric (official NGINX implementation)
  - Istio Gateway
  - Envoy Gateway
  - Kong Gateway
