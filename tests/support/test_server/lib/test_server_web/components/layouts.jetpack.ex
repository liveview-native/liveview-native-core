defmodule TestServerWeb.Layouts.Jetpack do
  use TestServerNative, [:layout, format: :jetpack]

  embed_templates "layouts_jetpack/*"
end
