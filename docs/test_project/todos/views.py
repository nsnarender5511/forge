from rest_framework import viewsets, filters, status, generics
from rest_framework.decorators import action
from rest_framework.response import Response
from rest_framework.permissions import IsAuthenticated
from .models import Todo
from .serializers import (
    TodoSerializer, 
    TodoCreateSerializer,
    TodoListSerializer
)

class TodoViewSet(viewsets.ModelViewSet):
    """
    API viewset for Todo items
    """
    serializer_class = TodoSerializer
    permission_classes = [IsAuthenticated]
    filter_backends = [filters.SearchFilter, filters.OrderingFilter]
    search_fields = ['title', 'description']
    ordering_fields = ['due_date', 'priority', 'status', 'created_at', 'updated_at']
    
    def get_queryset(self):
        """Return only todos belonging to the current user"""
        return Todo.objects.filter(user=self.request.user)
    
    def get_serializer_class(self):
        """Return appropriate serializer class based on action"""
        if self.action == 'create':
            return TodoCreateSerializer
        elif self.action == 'list':
            return TodoListSerializer
        return TodoSerializer
    
    def perform_create(self, serializer):
        """Save the authenticated user with the todo item"""
        serializer.save(user=self.request.user)
    
    @action(detail=False, methods=['get'])
    def completed(self, request):
        """Return all completed todos"""
        todos = self.get_queryset().filter(status='completed')
        page = self.paginate_queryset(todos)
        if page is not None:
            serializer = TodoListSerializer(page, many=True)
            return self.get_paginated_response(serializer.data)
        serializer = TodoListSerializer(todos, many=True)
        return Response(serializer.data)
    
    @action(detail=False, methods=['get'])
    def pending(self, request):
        """Return all pending todos"""
        todos = self.get_queryset().filter(status='pending')
        page = self.paginate_queryset(todos)
        if page is not None:
            serializer = TodoListSerializer(page, many=True)
            return self.get_paginated_response(serializer.data)
        serializer = TodoListSerializer(todos, many=True)
        return Response(serializer.data)
    
    @action(detail=True, methods=['patch'])
    def mark_completed(self, request, pk=None):
        """Mark a todo item as completed"""
        todo = self.get_object()
        todo.status = 'completed'
        todo.save()
        return Response({'status': 'todo marked as completed'})
    
    @action(detail=True, methods=['patch'])
    def mark_pending(self, request, pk=None):
        """Mark a todo item as pending"""
        todo = self.get_object()
        todo.status = 'pending'
        todo.save()
        return Response({'status': 'todo marked as pending'})
        
    @action(detail=False, methods=['delete'])
    def clear_completed(self, request):
        """Delete all completed todos"""
        todos = self.get_queryset().filter(status='completed')
        count = todos.count()
        todos.delete()
        return Response({'status': f'{count} completed todos deleted'}, 
                        status=status.HTTP_204_NO_CONTENT)


# Simple API Views for alternative access patterns
class TodoListCreateAPIView(generics.ListCreateAPIView):
    """
    API view to list and create Todo items
    """
    serializer_class = TodoSerializer
    permission_classes = [IsAuthenticated]
    
    def get_queryset(self):
        return Todo.objects.filter(user=self.request.user)
    
    def get_serializer_class(self):
        if self.request.method == 'POST':
            return TodoCreateSerializer
        return TodoListSerializer
        
    def perform_create(self, serializer):
        serializer.save(user=self.request.user)


class TodoDetailAPIView(generics.RetrieveUpdateDestroyAPIView):
    """
    API view to retrieve, update or delete a Todo item
    """
    serializer_class = TodoSerializer
    permission_classes = [IsAuthenticated]
    
    def get_queryset(self):
        return Todo.objects.filter(user=self.request.user)