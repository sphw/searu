<script>
	import {
		Nav,
		NavLink,
		NavItem,
        Tooltip
	} from 'sveltestrap/src';

  export let currentPath;
  export let path;
  export let title;
  let active = currentPath == path || (currentPath == "" && path == "/");
</script>

<NavItem>
	<NavLink class="py-3 border-bottom {active ? '' : 'link-dark'}" id="{title.replace(' ', '')}" href="{path}" active="{active ? 'true' : ''}">
        <slot></slot>
    </NavLink>
    <Tooltip placement="right" target="{title.replace(' ', '')}">
      {title}
    </Tooltip>
</NavItem>
