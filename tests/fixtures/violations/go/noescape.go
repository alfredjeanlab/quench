package violations

// VIOLATION: //go:noescape without NOESCAPE comment
//go:noescape
func fastHash(data []byte) uint64
