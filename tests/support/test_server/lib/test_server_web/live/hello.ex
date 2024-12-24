defmodule TestServerWeb.HelloLive do
  use TestServerWeb, :live_view
  use TestServerNative, :live_view

  def mount(_params, _session, socket) do
    socket =
      put_cookies(socket, [
        {"cookie_one", "value1", [http_only: true, max_age: 3600]},
        {"cookie_two", "value2", [http_only: true, max_age: 7200]}
      ])

    {:ok, socket}
  end

  defp put_cookies(socket, cookies) do
    Enum.reduce(cookies, socket, fn {key, value, opts}, acc ->
      Plug.Conn.put_resp_cookie(acc, key, value, opts)
    end)
  end

  def render(assigns) do
    ~H"""
    <p> FOOBAR </p>
    """
  end
end

defmodule TestServerWeb.HelloLive.Jetpack do
  use TestServerNative, [:render_component, format: :jetpack]

  def render(assigns, _) do
    ~LVN"""
    <Box size="fill" background="system-blue">
      <Text align="Center">Hello</Text>
    </Box>
    """
  end
end

defmodule TestServerWeb.HelloLive.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <VStack>
      <Text>
        Hello SwiftUI!
      </Text>
    </VStack>
    """
  end
end
