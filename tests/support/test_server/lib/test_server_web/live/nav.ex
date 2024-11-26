defmodule TestServerWeb.NavLive do
  use TestServerWeb, :live_view
  use TestServerNative, :live_view

  def mount(%{"dynamic" => path}, _session, socket) do
    {:ok, assign(socket, path: path)}
  end

  def render(assigns) do
    ~H"""
    <p>
      <%= @path %>
      <.link href={~p"/nav/next"}>Next</.link>
    </p>
    """
  end
end

defmodule TestServerWeb.NavLive.Jetpack do
  use TestServerNative, [:render_component, format: :jetpack]

  def render(assigns, _) do
    ~LVN"""
    <Box size="fill" background="system-blue">
      <Text align="Center">
        <%= @path %>
        <Link destination="/nav/next">
             <Text class="bold">Next</Text>
        </Link>
      </Text>
    </Box>
    """
  end
end

defmodule TestServerWeb.NavLive.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <VStack>
      <Text>
         <%= @path %>
      </Text>
      <NavigationLink id="Next" destination={"/nav/next"}>
      <Text>
      NEXT
      </Text>
      </NavigationLink>
    </VStack>
    """
  end
end
