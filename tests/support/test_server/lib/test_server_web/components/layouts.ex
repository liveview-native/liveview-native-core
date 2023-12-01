defmodule TestServerWeb.Layouts do
  use TestServerWeb, :html
  use LiveViewNative.Layouts

  embed_templates "layouts/*"
end
