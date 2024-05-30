defmodule TestServerWeb.Layouts.SwiftUI do
  use TestServerNative, [:layout, format: :swiftui]

  embed_templates "layouts_swiftui/*"
end
