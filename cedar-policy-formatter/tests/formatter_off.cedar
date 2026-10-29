permit(principal,action,resource)when{principal.a==1};

// @formatter:off
permit(principal  ==  User::"alice",
       action,
       resource)   // kept as written
  when { resource.owner   ==   principal };
forbid(principal,action,resource);
// @formatter:on

permit(principal,action,resource)when{"// @formatter:off"=="x"};

// @formatter:off
@id("last")
permit(
    principal,
    action,
    resource
);
