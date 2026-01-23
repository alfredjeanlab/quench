package violations

import _ "unsafe"

// VIOLATION: //go:linkname without LINKNAME comment
//go:linkname runtimeNano runtime.nanotime
func runtimeNano() int64
