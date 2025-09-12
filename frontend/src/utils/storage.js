export function getToken() {
  return localStorage.getItem('token')
}

export function setToken(token) {
  localStorage.setItem('token', token)
}

export function clearToken() {
  localStorage.removeItem('token')
}

export function isAdmin() {
  return localStorage.getItem('isAdmin') === 'true'
}

export function setIsAdmin(flag) {
  localStorage.setItem('isAdmin', flag ? 'true' : 'false')
}
