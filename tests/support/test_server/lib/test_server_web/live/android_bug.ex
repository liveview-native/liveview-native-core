defmodule TestServerWeb.AndroidBug do
  use TestServerWeb, :live_view
  use TestServerNative, :live_view

  def mount(_params, _session, socket) do
    {:ok, socket |> assign(:showDialog, false)}
  end

  def handle_event("showDialog", _params, socket) do
    {:noreply, socket |> assign(:showDialog, true)}
  end

  def handle_event("hideDialog", _params, socket) do
    {:noreply, socket |> assign(:showDialog, false)}
  end

  def render(assigns) do
    ~H"""
    <p>try again on android</p>
    <p>The dialog toggle is <%= @showDialog %></p>
    <%= if @showDialog do %>
      <button phx-click="hideDialog"> Hide </button>
    <% else %>
      <button phx-click="showDialog"> Show </button>
    <% end %>
    """
  end
end

defmodule TestServerWeb.AndroidBug.Jetpack do
  use TestServerNative, [:render_component, format: :jetpack]

  def render(assigns, _) do
    ~LVN"""
      <Column width="fill" verticalArrangement="center" horizontalAlignment="center" scroll="vertical">
        <OutlinedButton phx-click="showDialog"><Text>Show Dialog</Text></OutlinedButton>
        <%= if @showDialog do %>
        <AlertDialog phx-click="hideDialog">
          <ConfirmButton>
              <TextButton  phx-click="hideDialog">
                <Text>Confirm</Text>
              </TextButton>
          </ConfirmButton>
          <ConfirmButton>
              <TextButton  phx-click="hideDialog">
                <Text>Confirm</Text>
              </TextButton>
          </ConfirmButton>
          <DismissButton>
            <OutlinedButton phx-click="hideDialog">
              <Text>Dismiss</Text>
            </OutlinedButton>
          </DismissButton>
          <Icon imageVector="filled:Add" />
          <Title>Alert Title</Title>
          <Content>
              <Text>Alert message</Text>
          </Content>
        </AlertDialog>
        <% end %>
        <Box size="100" contentAlignment="center" />
      </Column>
    """
  end
end

defmodule TestServerWeb.AndroidBug.SwiftUI do
  use TestServerNative, [:render_component, format: :swiftui]

  def render(assigns, _interface) do
    ~LVN"""
    <VStack>
      <Text>
        You are not supposed to be here it's a bug repro for someone else
      </Text>
    </VStack>
    """
  end
end
