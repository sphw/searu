
<script type="ts">
  import { session } from '$app/stores';
  import { goto } from '$app/navigation';

  let username = '';
  let password = '';
  let error = null;

  async function submit() {
    const response = await fetch(`users/login`, {
          method: 'POST',
          credentials: 'include',
          body: JSON.stringify({ username, password }),
          headers: {
              'Content-Type': 'application/json'
          }
      }).then((r) => r.json());
    // TODO handle network errors
    error = response.msg;
    if (response.token) {
      session.jwt = response.token;
      goto('/');
    }
}
</script>
{#if error}
  <div class="alert alert-danger" role="alert">
    {error}
  </div>
{/if}
<div class="form-signin">
  <form on:submit|preventDefault={submit}>
    <svg xmlns="http://www.w3.org/2000/svg" width="72" height="57" fill="currentColor" class="bi bi-app mb-4" viewBox="0 0 16 16">
      <path d="M11 2a3 3 0 0 1 3 3v6a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3V5a3 3 0 0 1 3-3h6zM5 1a4 4 0 0 0-4 4v6a4 4 0 0 0 4 4h6a4 4 0 0 0 4-4V5a4 4 0 0 0-4-4H5z"/>
    </svg>

    <h1 class="h3 mb-3 fw-normal">Please sign in</h1>

    <div class="form-floating">
      <input type="username" class="form-control" id="floatingInput" placeholder="admin" bind:value={username}>
      <label for="floatingInput">Username</label>
    </div>
    <div class="form-floating">
      <input type="password" class="form-control" id="floatingPassword" placeholder="Password" bind:value={password}>
      <label for="floatingPassword">Password</label>
    </div>

    <div class="checkbox mb-3">
    </div>
    <button class="w-100 btn btn-lg btn-primary" type="submit">Sign in</button>
  </form>
</div>
