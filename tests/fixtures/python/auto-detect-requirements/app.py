"""Application module."""


def create_app():
    """Create the application."""
    return {"name": "myapp"}


def run():
    """Run the application."""
    app = create_app()
    print(f"Running {app['name']}")
